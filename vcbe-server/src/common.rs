use rand::{Rng, thread_rng};
use rocket_db_pools::{Connection, sqlx};
use std::collections::HashMap;
use rocket_db_pools::sqlx::Row;
use vcbe_core::{Evidence, LV_RANGES};
use crate::{Base, WithConn};

pub fn choose_word_common(history: &[(u32, bool)], lv: usize) -> u32 {
    let mut current_word = thread_rng().gen_range(LV_RANGES[lv].clone());
    while history.iter().any(|(x, _)| *x == current_word) {
        current_word = thread_rng().gen_range(LV_RANGES[lv].clone());
    }
    current_word
}

pub async fn result_common(
    history: &[(u32, bool)], mut db: Connection<Base>
) -> WithConn<HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut evidences = Vec::with_capacity(history.len());
    for (i, correct) in history {
        let row = sqlx::query("SELECT freq, lv FROM words WHERE id = ?")
            .bind(i).fetch_one(&mut **db).await.unwrap();
        let freq: u32 = row.get::<i32, _>(0) as u32;
        let lv: u8 = row.get::<i32, _>(1) as u8;
        evidences.push(Evidence {
            id: *i as usize,
            freq,
            lv,
            correct: *correct,
        });
    }
    let est_uls = vcbe_core::estimate_uls(evidences.clone());
    result.insert("uls".to_string(), est_uls.to_string());
    let est_rfwls = vcbe_core::estimate_rfwls(evidences.clone());
    result.insert("rfwls".to_string(), est_rfwls.to_string());
    let freq = {
        let rows = sqlx::query("SELECT freq FROM words")
            .fetch_all(&mut **db).await.unwrap();
        rows.iter()
            .map(|row| row.get::<i32, _>(0) as u32)
            .collect::<Vec<u32>>()
    };
    let est_mle = vcbe_core::estimate_mle(evidences, freq);
    result.insert("mle".to_string(), est_mle.to_string());
    (result, db)
}
