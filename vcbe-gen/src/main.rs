use std::collections::{HashMap, HashSet};
use std::{fs, iter};
use std::fmt::Write;
use std::io::Cursor;
use indicatif::{ProgressState, ProgressStyle};
use log::info;
use vcbe_core::Word;
use rapidfuzz::distance::levenshtein;
use rayon::prelude::*;

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct OrgWord {
    word: String,
    headword: String,
    frequency: String,
    list: String,
    usPhone: String,
    ukPhone: String,
    examType: Vec<String>,
    translations: Vec<String>,
    phrs: Vec<Phrase>,
    sentences: Vec<Sentence>
}

#[derive(serde::Deserialize)]
struct Phrase {
    headword: String,
    translation: String
}

#[derive(serde::Deserialize)]
struct Sentence {
    sentence: String,
    translation: String,
}

fn main_migrate() {
    info!("Reading words from BNC_COCA_EN2CN.");
    let words = fs::read_dir("../BNC_COCA_EN2CN/data")
        .expect("Error reading dictionary data.")
        .map(|x| x.unwrap())
        .map(|x| fs::read(x.path()).unwrap())
        .map(|x| serde_json::from_slice(&x).unwrap())
        .collect::<Vec<OrgWord>>();
    info!("Read {} words from BNC_COCA_EN2CN.", words.len());
    info!("Migrating to internal format.");
    let words = words.into_iter().map(migrate).collect::<Vec<Word>>();
    info!("Serializing into MessagePack format.");
    let rmp = rmp_serde::to_vec(&words).unwrap();
    info!("Compressing RMP binary.");
    let rmp_zstd = zstd::encode_all(Cursor::new(rmp), 19).unwrap();
    info!("Saving compressed RMP dictionary.");
    fs::write("dict.rmp.zstd", rmp_zstd).unwrap()
}

fn main_zero_freq() {
    let words: Vec<Word> = rmp_serde::from_slice(&zstd::decode_all(
        Cursor::new(fs::read("dict.rmp.zstd").unwrap())).unwrap()).unwrap();
    let zero_freq = words.iter()
        .filter(|x| x.freq == 0)
        .cloned().collect::<Vec<_>>();
    info!("There are {} words with frequency 0 out of {} words.", zero_freq.len(), words.len());
    let zero_freq: HashSet<vcbe_core::Word> = HashSet::from_iter(zero_freq.into_iter());
    info!("Filtering out obvious plural forms.");
    let mut words = words.into_iter()
        .filter(|x| !zero_freq.contains(x))
        .collect::<Vec<_>>();
    let pool = words.iter().map(|x| x.word.clone()).collect::<HashSet<_>>();
    let zero_freq = zero_freq.into_iter()
        .filter(|x| {
            !pool.contains(&x.head)
        })
        .collect::<Vec<_>>();
    info!("Remaining {} unique words with frequency 0.", zero_freq.len());
    let rmp = rmp_serde::to_vec(&words).unwrap();
    info!("Compressing RMP binary.");
    let rmp_zstd = zstd::encode_all(Cursor::new(rmp), 19).unwrap();
    info!("Saving compressed RMP dictionary.");
    fs::write("dict.rmp.zstd", rmp_zstd).unwrap()
}

fn main_entry_gen() {
    let (words, levels) = main_entry_parts();
    let lev_dist = main_entry_lev_dist(&words);
    let incl = main_entry_incl(&words);
    let incl_rev = main_entry_incl_rev(&incl);
}

fn main_entry_parts() -> (Vec<Word>, Vec<u8>) {
    const PARTS: [usize; 8] = [1023, 1902, 3595, 6562, 10251, 13612, 12300, 18933];
    let mut words: Vec<Word> = rmp_serde::from_slice(&zstd::decode_all(
        Cursor::new(fs::read("dict.rmp.zstd").unwrap())).unwrap()).unwrap();
    words.sort_unstable_by_key(|x| x.freq);
    let mut parts = Vec::with_capacity(8);
    let mut begin = 0;
    for len in PARTS {
        let end = begin + len;
        let mut part = words[begin..end].to_vec();
        part.sort_unstable_by_key(|x| x.freq);
        parts.push(part);
        begin = end;
    }
    assert_eq!(begin, words.len());
    let density = parts.iter()
        .map(|x| x.iter().map(|x| x.freq as u64).sum::<u64>())
        .collect::<Vec<_>>();
    info!("Density: {:?}", density);
    let parts = parts.into_iter()
        .enumerate()
        .flat_map(|(i, x)| x.into_iter()
            .zip(iter::repeat(i as u8)))
        .collect::<Vec<_>>();
    let levels = parts.iter().map(|x| x.1).collect::<Vec<_>>();
    let words = parts.into_iter().map(|x| x.0).collect::<Vec<_>>();
    (words, levels)
}

fn main_entry_lev_dist(words: &[Word]) -> Vec<Vec<(usize, usize)>> {
    info!("Collecting similar words.");
    let args = levenshtein::Args::default()
        .score_cutoff(3)
        .score_hint(3);
    let pb = indicatif::ProgressBar::new(words.len() as u64);
    words.par_iter().map(|word| {
        let similar = words.iter()
            .enumerate()
            .filter_map(|(i, x)|
            levenshtein::distance_with_args(word.word.chars(), x.word.chars(), &args)
                .map(|y| (i, y)))
            .take(50)
            .collect::<Vec<_>>();
        pb.inc(1);
        similar
    }).collect::<Vec<_>>()
}

fn main_entry_incl(words: &[Word]) -> Vec<Vec<usize>> {
    info!("Collecting inter-entry inclusions.");
    let mut words = words.into_iter().enumerate().collect::<Vec<_>>();
    words.sort_unstable_by_key(|(i, w)| w.word.len());
    let shortest_len = words[0].1.word.len();
    let longest_len = words.last().unwrap().1.word.len();
    let first_longer_idx = (0..=(longest_len-shortest_len))
        .map(|x| {
            let len = x + shortest_len;
            let index = words.iter()
                .position(|(i, w)| w.word.len() > len);
            index.unwrap_or(usize::MAX)
        })
        .collect::<Vec<_>>();
    let pb = indicatif::ProgressBar::new(words.len() as u64);
    words.par_iter().map(|(ii, w)| {
        let len = w.word.len();
        let begin = first_longer_idx[len - shortest_len];
        let mut res = if begin != usize::MAX && len > 2 {
            words[begin..].iter().filter_map(|(i, ww)| {
                if ww.word.contains(&w.word) && ii != i { Some(*i) } else { None }
            }).take(50).collect()
        } else {
            Vec::new()
        };
        pb.inc(1);
        res
    }).collect()
}

fn main_entry_incl_rev(incl: &[Vec<usize>]) -> Vec<Vec<usize>> {
    info!("Collecting reverse inter-entry inclusions.");
    let mut incl_rev = vec![Vec::new(); incl.len()];
    incl.iter().enumerate().for_each(|(i, x)| {
        x.iter().for_each(|&j| {
            incl_rev[j].push(i);
        });
    });
    incl_rev
}

fn main() {
    env_logger::init();
    info!("Vocabble Database Generation Utility");
    main_entry_gen();
}

fn migrate(word: OrgWord) -> Word {
    Word {
        word: word.word,
        head: word.headword,
        freq: word.frequency.parse().unwrap(),
        list: word.list,
        p_us: word.usPhone,
        p_uk: word.ukPhone,
        exam: word.examType,
        desc: word.translations,
        phr: word.phrs.iter().map(|x| x.headword.clone()).collect(),
        phr_desc: word.phrs.iter().map(|x| x.translation.clone()).collect(),
        sen: word.sentences.iter().map(|x| x.sentence.clone()).collect(),
        sen_desc: word.sentences.iter().map(|x| x.translation.clone()).collect(),
    }
}
