use std::collections::HashMap;
use std::ops::Range;
#[cfg(feature = "tyv")]
use tch::{CModule, Tensor};

pub const LV_RANGES: [Range<u32>; 8] = [
    0..1023, 1023..2925, 2925..6520, 6520..13082,
    13082..23333, 23333..36945, 36945..49245, 49245..68178
];
pub const LV_COUNTS: [usize; 8] = [1023, 1902, 3595, 6562, 10251, 13612, 12300, 18933];

#[derive(serde::Serialize, serde:: Deserialize)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Word {
    pub word: String,
    pub head: String,
    pub freq: u32,
    pub list: String,
    pub p_us: String,
    pub p_uk: String,
    pub exam: Vec<String>,
    pub desc: Vec<String>,
    pub phr: Vec<String>,
    pub phr_desc: Vec<String>,
    pub sen: Vec<String>,
    pub sen_desc: Vec<String>,
}

#[derive(serde::Serialize, serde:: Deserialize)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Entry {
    pub word: String,
    pub head: String,
    pub freq: u32,
    pub list: String,
    pub p_us: String,
    pub p_uk: String,
    pub exam: Vec<String>,
    pub desc: Vec<String>,
    pub phr: Vec<String>,
    pub phr_desc: Vec<String>,
    pub sen: Vec<String>,
    pub sen_desc: Vec<String>,
    pub lv: u8,
    pub sim: Vec<usize>,
    pub incl: Vec<usize>,
    pub incl_rev: Vec<usize>,
}

#[derive(serde::Serialize, serde:: Deserialize)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Row {
    pub id: usize,
    pub word: String,
    pub freq: u32,
    pub desc: Vec<String>,
    pub lv: u8,
    pub sim: Vec<usize>,
    pub incl: Vec<usize>,
    pub incl_rev: Vec<usize>,
}

#[derive(serde::Serialize, serde:: Deserialize)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Message {
    pub session: u32,
    pub details: HashMap<String, String>,
}

#[derive(serde::Serialize, serde:: Deserialize)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Evidence {
    pub id: usize,
    pub freq: u32,
    pub lv: u8,
    pub correct: bool,
}

/// Uniform leveled scaling
pub fn estimate_uls(evidences: Vec<Evidence>) -> usize {
    // count the number of answers & ~ of correct answers for each level
    let ratios = evidences.iter().fold([(0, 0); 8], |mut acc, x| {
        acc[x.lv as usize].0 += 1u32;
        acc[x.lv as usize].1 += x.correct as usize;
        acc
    });
    let mut estimate = 0u32;
    // assume every word can represent each other within the same level
    for (i, (total, correct)) in ratios.iter().enumerate() {
        if *total == 0 { continue; }
        estimate += (LV_COUNTS[i] * *correct / *total as usize) as u32;
    }
    estimate as usize
}

/// Reciprocal frequency weighted leveled scaling
pub fn estimate_rfwls(evidences: Vec<Evidence>) -> usize {
    let one = u128::MAX / 1000_0000;
    // same as ULS but weight with the reciprocal of frequency
    let ratios = evidences.iter().fold([(0, 0); 8], |mut acc, x| {
        let weight = one / x.freq as u128;
        acc[x.lv as usize].0 += weight;
        acc[x.lv as usize].1 += x.correct as u128 * weight;
        acc
    });
    let mut estimate = 0u32;
    for (i, (total, correct)) in ratios.iter().enumerate() {
        if *total == 0 { continue; }
        estimate += (LV_COUNTS[i] as u128 * *correct / *total) as u32;
    }
    estimate as usize
}

/// Maximum likelihood estimation
// pub fn estimate_mle(evidence: Vec<Evidence>, freq: Vec<u32>) -> usize {
//     let freq_total = *freq.iter().max().unwrap() as f64 / 1000.0;
//
//     let likelihood = |est: f64, freq_known: &[f64], freq_unknown: &[f64]| -> f64 {
//         let prod_known = freq_known.iter()
//             .map(|x| x / freq_total)
//             .map(|x| 1.0 - (1.0 - x).powf(est))
//             .fold(1.0, |acc, x| acc * x);
//         let prod_unknown = freq_unknown.iter()
//             .map(|x| x / freq_total)
//             .map(|x| 1.0 - x.powf(est))
//             .fold(1.0, |acc, x| acc * x);
//         prod_known * prod_unknown
//     };
//
//     let freq_known: Vec<f64> = evidence.iter()
//         .filter(|x| x.correct)
//         .map(|x| x.freq as f64)
//         .collect();
//     let freq_unknown: Vec<f64> = evidence.iter()
//         .filter(|x| !x.correct)
//         .map(|x| x.freq as f64)
//         .collect();
//
//     let word_seen = (0..60_0000).step_by(1000)
//         .map(|x| likelihood(x as f64 / 1000.0, &freq_known, &freq_unknown))
//         .max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();
//
//     let est = freq.iter()
//         .map(|x| *x as f64 / freq_total)
//         .map(|x| (x - 1.0) * ((1.0 - x).powf(word_seen) - 1.0))
//         .sum::<f64>();
//     est as usize
// }

/// Heuristic estimation
pub fn estimate_heu(evidences: Vec<Evidence>) -> usize {
    // let max = 30.0;
    // let mut spectrum = [0f64; 68178];
    // for word in evidences {
    //     let weight = max / (word.freq as f64).log2();
    //     spectrum.iter_mut().enumerate()
    //         .filter(|(i, _)| (*i as isize - word.id as isize).abs() < 50 * weight as isize)
    //         .for_each(|(i, x)|
    //         *x += weight * (1.0 - (i as isize - word.id as isize).abs() as f64 / 50.0));
    // }
    // spectrum.iter().enumerate()
    //     .max_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap())
    //     .unwrap().0
    // let total = 9056.0;
    
    // use a large number as "one" to avoid precision loss
    let one = u128::MAX / 1000_0000;
    let total: u128 = evidences.iter()
        .map(|x| one / x.freq as u128).sum();
    let correct: u128 = evidences.iter()
        .filter(|x| x.correct)
        .map(|x| one / x.freq as u128).sum();
    (correct * 68178 / total) as usize
}

/// Machine learning based mimicry of Test-Your-Vocab scoring
#[cfg(feature = "tyv")]
pub fn estimate_tyv(result: &[(&str, bool)], data: &TyvData) -> usize {
    let mut broad = [0.0; 127];
    let mut narrow = [0.0; 608];
    // in each vector, 1.0 for correct, -1.0 for incorrect, 0.0 for unknown (not tested)
    for (word, recall) in result {
        if let Some(i) = data.broad_toi.get(&word.to_string()) {
            broad[*i] = if *recall { 1.0 } else { -1.0 }
        }
        if let Some(i) = data.narrow_toi.get(&word.to_string()) {
            narrow[*i] = if *recall { 1.0 } else { -1.0 }
        }
    }
    tyv_inference(&data.model, &broad, &narrow) as usize
}

#[cfg(feature = "tyv")]
pub struct TyvData {
    pub broad_toi: HashMap<String, usize>,
    pub narrow_toi: HashMap<String, usize>,
    pub model: CModule,
}

#[cfg(feature = "tyv")]
fn tyv_inference(model: &CModule, broad: &[f32], narrow: &[f32]) -> f32 {
    let broad = Tensor::from_slice2(&[broad]);
    let narrow = Tensor::from_slice2(&[narrow]);
    // run inference on the model, outputting a scalar representing the estimation of the
    // "known ratio" of the corpus
    let output = model.forward_ts(&[broad, narrow]).unwrap();
    (output.double_value(&[0, 0]) * 45000.0) as f32
}
