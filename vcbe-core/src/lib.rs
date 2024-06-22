use std::collections::HashMap;
use std::ops::Range;

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
    let ratios = evidences.iter().fold([(0, 0); 8], |mut acc, x| {
        acc[x.lv as usize].0 += 1u32;
        acc[x.lv as usize].1 += x.correct as usize;
        acc
    });
    let mut estimate = 0u32;
    for (i, (total, correct)) in ratios.iter().enumerate() {
        estimate += (LV_COUNTS[i] * *correct / *total as usize) as u32;
    }
    estimate as usize
}

/// Reciprocal frequency weighted leveled scaling
pub fn estimate_rfwls(evidences: Vec<Evidence>) -> usize {
    let one = u128::MAX / 1000_0000;
    let ratios = evidences.iter().fold([(0, 0); 8], |mut acc, x| {
        let weight = one / x.freq as u128;
        acc[x.lv as usize].0 += weight;
        acc[x.lv as usize].1 += x.correct as u128 * weight;
        acc
    });
    let mut estimate = 0u32;
    for (i, (total, correct)) in ratios.iter().enumerate() {
        estimate += (LV_COUNTS[i] as u128 * *correct / *total) as u32;
    }
    estimate as usize
}

/// Maximum likelihood estimation
pub fn estimate_mle(evidence: Vec<Evidence>, freq: Vec<u32>) -> usize {
    let freq_total = freq.iter().sum::<u32>() as f64;

    let likelihood = |est: f64, freq_known: &[f64], freq_unknown: &[f64]| -> f64 {
        let prod_known = freq_known.iter()
            .map(|x| x / freq_total)
            .map(|x| 1.0 - (1.0 - x).powf(est))
            .fold(1.0, |acc, x| acc * x);
        let prod_unknown = freq_unknown.iter()
            .map(|x| x / freq_total)
            .map(|x| 1.0 - x.powf(est))
            .fold(1.0, |acc, x| acc * x);
        prod_known * prod_unknown
    };

    let freq_known: Vec<f64> = evidence.iter()
        .filter(|x| x.correct)
        .map(|x| x.freq as f64)
        .collect();
    let freq_unknown: Vec<f64> = evidence.iter()
        .filter(|x| !x.correct)
        .map(|x| x.freq as f64)
        .collect();

    let word_seen = (0..60_0000)
        .map(|x| likelihood(x as f64 / 1000.0, &freq_known, &freq_unknown))
        .max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();

    let est = freq.iter()
        .map(|x| *x as f64 / freq_total)
        .map(|x| (x - 1.0) * ((1.0 - x).powf(word_seen) - 1.0))
        .sum::<f64>();
    est as usize
}

/// Machine learning based mimicry of Test-Your-Vocab scoring
pub fn estimate_tyv(evidence: Vec<Evidence>) -> usize {
}

pub struct TyvData {
    pub words: Vec<String>,
    pub toi: HashMap<String, usize>,
    pub
}