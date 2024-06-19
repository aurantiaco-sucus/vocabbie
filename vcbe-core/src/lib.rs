
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