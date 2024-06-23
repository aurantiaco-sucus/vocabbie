use std::collections::HashMap;
use std::env::args;
use std::fs;
use std::process::exit;
use rayon::prelude::*;
use vcbe_core::{Evidence, Row};

fn main() {
    let data: Vec<Row> = rmp_serde::from_slice(&fs::read("rows.rmp").unwrap()).unwrap();
    // main_weight_density(&data);
    let dict = data.iter()
        .map(|r| (r.word.clone(), r.clone()))
        .collect::<HashMap<_, _>>();
    let freq = data.iter().map(|r| r.freq).collect::<Vec<_>>();
    let cases = args().nth(1).unwrap();
    let cases = fs::read_to_string(cases).unwrap();
    let cases = cases.lines()
        .filter(|x| !x.is_empty())
        .map(|x| x.trim())
        .map(|x| x.split_once(';').unwrap())
        .map(|(k, u)| (k.split(',').collect::<Vec<_>>(), 
                       u.split(',').collect::<Vec<_>>()))
        .collect::<Vec<_>>();
    let pb = indicatif::ProgressBar::new(cases.len() as u64);
    let results = cases.par_iter()
        .map(|(k, u)| { 
            let res = process(&dict, k, u);
            pb.inc(1);
            res
        })
        .map(|(uls, rfwls, mle)| format!("{},{},{}", uls, rfwls, mle))
        .collect::<Vec<_>>().join("\n");
    let target = args().nth(2).unwrap();
    fs::write(target, results).unwrap();
}

fn process(
    dict: &HashMap<String, Row>, known: &[&str], unknown: &[&str]
) -> (usize, usize, usize) {
    let evidences: Vec<Evidence> = {
        known.iter().filter_map(|k| {
            let row = dict.get(*k)?;
            Evidence {
                id: row.id,
                freq: row.freq,
                lv: row.lv,
                correct: true,
            }.into()
        }).chain(unknown.iter().filter_map(|k| {
            let row = dict.get(*k)?;
            Evidence {
                id: row.id,
                freq: row.freq,
                lv: row.lv,
                correct: false,
            }.into()
        })).collect()
    };
    if evidences.is_empty() {
        return (0, 0, 0);
    }
    let uls = vcbe_core::estimate_uls(evidences.clone());
    let rfwls = vcbe_core::estimate_rfwls(evidences.clone());
    //let mle = vcbe_core::estimate_mle(evidences.clone(), freq);
    let heu = vcbe_core::estimate_heu(evidences.clone());
    (uls, rfwls, heu)
}

// fn main_weight_density(data: &[Row]) {
//     let one = u128::MAX / 1000_0000;
//     let den: u128 = data.iter().map(|r| one / ((r.freq as u128).ilog2() as u128 + 1)).sum();
//     let den = den / one;
//     println!("{}", den);
//     exit(0);
// }