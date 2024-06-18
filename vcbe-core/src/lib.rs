
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