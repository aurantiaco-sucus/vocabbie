use std::collections::HashMap;
use std::ops::Range;
use rand::prelude::*;
use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;
use vcbe_core::{Evidence, LV_RANGES, Message};
use crate::{Base, BaseConn, WithConn};

pub struct Session {
    pub history: Vec<(u32, bool)>,
    pub current_word: u32,
    pub question: String,
    pub candidates: Vec<String>,
    pub answer: usize,
}

pub async fn create(db: BaseConn) -> Session {
    let mut session = Session {
        history: Vec::new(),
        current_word: 0,
        question: "".to_string(),
        candidates: Vec::new(),
        answer: 0,
    };
    let _ = update(&mut session, db).await;
    session
}

async fn related(word: u32, mut db: BaseConn) -> WithConn<Vec<u32>> {
    let row = sqlx::query(
        "SELECT sim, incl, incl_rev, lv FROM words WHERE id = ?")
        .bind(word)
        .fetch_one(&mut **db).await.unwrap();
    let sim: Vec<u32> = {
        let sim: String = row.get(0);
        sim.split(',').filter_map(|x| x.parse().ok()).collect()
    };
    let incl: Vec<u32> = {
        let incl: String = row.get(1);
        incl.split(',').filter_map(|x| x.parse().ok()).collect()
    };
    let incl_rev: Vec<u32> = {
        let incl_rev: String = row.get(2);
        incl_rev.split(',').filter_map(|x| x.parse().ok()).collect()
    };
    let mut related: Vec<u32> = sim.into_iter().chain(incl).chain(incl_rev).collect();
    if related.len() < 5 {
        let lv: u32 = row.get(3);
        while related.len() < 5 {
            let new_word = thread_rng().gen_range(LV_RANGES[lv as usize].clone());
            if !related.contains(&new_word) && new_word != word {
                related.push(new_word);
            }
        }
    }
    (related, db)
}

async fn gen_en2cn(
    word: u32, related: &[u32], mut db: BaseConn
) -> WithConn<(Vec<String>, usize)> {
    let mut sim_des = Vec::new();
    for &wi in related {
        let d: String = sqlx::query(
            "SELECT des FROM words WHERE id = ?")
            .bind(wi)
            .fetch_one(&mut **db).await.unwrap()
            .get(0);
        sim_des.extend(d.split(";;;")
            .map(|x| x.to_string()));
    }
    let real_des: Vec<String> = sqlx::query(
        "SELECT des FROM words WHERE id = ?")
        .bind(word)
        .fetch_one(&mut **db).await.unwrap()
        .get::<String, _>(0)
        .split(";;;")
        .map(|x| x.to_string())
        .collect();
    let mut choices: Vec<String> = sim_des
        .choose_multiple(&mut thread_rng(), 3)
        .cloned().collect();
    let correct_index = thread_rng().gen_range(0..4);
    choices.insert(correct_index, real_des.choose(&mut thread_rng()).unwrap().clone());
    ((choices, correct_index), db)
}

async fn gen_cn2en(
    word: u32, related: &[u32], mut db: BaseConn
) -> WithConn<(Vec<String>, usize, String)> {
    let mut sim_word = Vec::new();
    for &wi in related {
        let w: String = sqlx::query(
            "SELECT word FROM words WHERE id = ?")
            .bind(wi)
            .fetch_one(&mut **db).await.unwrap()
            .get(0);
        sim_word.push(w);
    }
    let real_word: String = sqlx::query(
        "SELECT word FROM words WHERE id = ?")
        .bind(word)
        .fetch_one(&mut **db).await.unwrap()
        .get(0);
    let mut choices: Vec<String> = sim_word
        .choose_multiple(&mut thread_rng(), 3)
        .cloned().collect();
    let correct_index = thread_rng().gen_range(0..4);
    choices.insert(correct_index, real_word.clone());
    let real_des: String = sqlx::query(
        "SELECT des FROM words WHERE id = ?")
        .bind(word)
        .fetch_one(&mut **db).await.unwrap()
        .get(0);
    let real_des = real_des.split(";;;").collect::<Vec<&str>>();
    let real_des = real_des.choose(&mut thread_rng()).unwrap().to_string();
    ((choices, correct_index, real_des), db)
}

pub async fn state(session: &Session, mut db: Connection<Base>) -> Json<Message> {
    let result_available = session.history.len() >= 24;
    let question = session.question.clone();
    let candidates = session.candidates.clone();
    Json(Message {
        session: 0,
        details: HashMap::from([
            ("result_available".to_string(), result_available.to_string()),
            ("question".to_string(), question),
            ("candidates".to_string(), candidates.join(";;;")),
        ]),
    })
}

pub async fn submit(
    session: &mut Session, db: BaseConn, data: Json<Message>
) -> (Json<Message>, bool) {
    match data.details.get("action") {
        None => (Json(Message {
            session: 0,
            details: HashMap::from([
                ("error".to_string(), "no action specified".to_string()),
            ]),
        }), false),
        Some(action) => match action as &str {
            "choose" => match data.details.get("choice") {
                None => (Json(Message {
                    session: 0,
                    details: HashMap::from([
                        ("error".to_string(), "no choice specified".to_string()),
                    ]),
                }), false),
                Some(choice) => {
                    let choice: usize = choice.parse().unwrap();
                    let correct = session.answer == choice;
                    session.history.push((session.current_word, correct));
                    update(session, db).await;
                    (Json(Message {
                        session: 0,
                        details: HashMap::from([
                            ("correct".to_string(), correct.to_string()),
                        ]),
                    }), false)
                }
            }
            "finish" => {
                if session.history.len() < 24 {
                    (Json(Message {
                        session: 0,
                        details: HashMap::from([
                            ("error".to_string(), "not enough questions answered".to_string()),
                        ]),
                    }), false)
                } else {
                    let (result, db) = result(session, db).await;
                    (Json(Message {
                        session: 0,
                        details: result,
                    }), true)
                }
            }
            _ => (Json(Message {
                session: 0,
                details: HashMap::from([
                    ("error".to_string(), "invalid action".to_string()),
                ]),
            }), false),
        }
    }
}

async fn update(session: &mut Session, db: BaseConn) -> BaseConn {
    let ordinal = session.history.len();
    let (lv, is_cn2en) = if ordinal < 24 {
        (ordinal / 3, ordinal % 3 == 1)
    } else {
        (((ordinal - 24) / 2) % 8, ordinal % 2 == 1)
    };
    let mut current_word = thread_rng().gen_range(LV_RANGES[lv].clone());
    while session.history.iter().any(|(x, _)| *x == current_word) {
        current_word = thread_rng().gen_range(LV_RANGES[lv].clone());
    }
    let (related, mut db) = related(current_word, db).await;
    let (question, candidates, answer, db) = if is_cn2en {
        let ((candidates, answer, question), db) = 
            gen_cn2en(current_word, &related, db).await;
        (question, candidates, answer, db)
    } else {
        let question = sqlx::query(
            "SELECT word FROM words WHERE id = ?")
            .bind(current_word)
            .fetch_one(&mut **db).await.unwrap().get(0);
        let ((candidates, answer), db) =
            gen_en2cn(current_word, &related, db).await;
        (question, candidates, answer, db)
    };
    session.current_word = current_word;
    session.question = question;
    session.candidates = candidates;
    session.answer = answer;
    db
}

async fn result(session: &Session, mut db: BaseConn) -> WithConn<HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut evidences = Vec::with_capacity(session.history.len());
    for (i, correct) in &session.history {
        let row = sqlx::query("SELECT freq, lv FROM words WHERE id = ?")
            .bind(i).fetch_one(&mut **db).await.unwrap();
        let freq: u32 = row.get(0);
        let lv: u8 = row.get(1);
        evidences.push(Evidence {
            id: *i as usize,
            freq,
            lv,
            correct: *correct,
        });
    }
    let est_uls = vcbe_core::estimate_uls(evidences.clone());
    result.insert("uls".to_string(), est_uls.to_string());
    let est_rfwls = vcbe_core::estimate_rfwls(evidences);
    result.insert("rfwls".to_string(), est_rfwls.to_string());
    (result, db)
}