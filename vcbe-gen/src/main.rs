use std::collections::{HashMap, HashSet};
use std::{fs, iter};
use std::io::Cursor;
use log::info;

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
    let words = words.into_iter().map(migrate).collect::<Vec<vcbe_core::Word>>();
    info!("Serializing into MessagePack format.");
    let rmp = rmp_serde::to_vec(&words).unwrap();
    info!("Compressing RMP binary.");
    let rmp_zstd = zstd::encode_all(Cursor::new(rmp), 19).unwrap();
    info!("Saving compressed RMP dictionary.");
    fs::write("dict.rmp.zstd", rmp_zstd).unwrap()
}

fn main_zero_freq() {
    let words: Vec<vcbe_core::Word> = rmp_serde::from_slice(&zstd::decode_all(
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
    let words = main_entry_gen_parts();
}

fn main_entry_gen_parts() -> Vec<(vcbe_core::Word, u8)> {
    const PARTS: [usize; 6] = [1023, 1902, 3595, 6562, 36163, 31233];
    let mut words: Vec<vcbe_core::Word> = rmp_serde::from_slice(&zstd::decode_all(
        Cursor::new(fs::read("dict.rmp.zstd").unwrap())).unwrap()).unwrap();
    words.sort_unstable_by_key(|x| x.freq);
    let mut parts = Vec::with_capacity(6);
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
    parts.into_iter()
        .enumerate()
        .flat_map(|(i, x)| x.into_iter()
            .zip(iter::repeat(i as u8)))
        .collect::<Vec<_>>()
}

fn main_entry_gen_similar() {
    
}

fn main() {
    env_logger::init();
    info!("Vocabble Database Generation Utility");
    main_entry_gen();
}

fn migrate(word: OrgWord) -> vcbe_core::Word {
    vcbe_core::Word {
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
