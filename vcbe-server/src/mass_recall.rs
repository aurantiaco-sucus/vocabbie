use std::collections::{BTreeMap, HashMap};
use once_cell::sync::Lazy;
use rand::{Rng, thread_rng};

use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use vcbe_core::{Message, TyvData};

use crate::{Base, BaseConn, common};

pub struct Session {
    pub history: Vec<(u32, bool)>,
    pub current_words: Vec<u32>,
}

pub async fn create(db: BaseConn) -> Session {
    let mut session = Session {
        history: Vec::new(),
        current_words: Vec::new(),
    };
    let _ = update(&mut session, db).await;
    session
}

async fn update(session: &mut Session, db: BaseConn) -> BaseConn {
    let ordinal = session.history.len();
    if session.tyv_mode {
        let range = if ordinal < 40 {
            0..TYV_BROAD.ito.len()
        } else {
            TYV_BROAD.ito.len()..TYV_BROAD.ito.len() + TYV_NARROW.ito.len()
        };
        let mut word = thread_rng().gen_range(range.clone());
        while session.history.iter().any(|(x, _)| *x == word as u32) {
            word = thread_rng().gen_range(range.clone());
        }
        session.current_word = word as u32;
        return db;
    }
    let lv = if ordinal < 24 {
        ordinal / 3
    } else {
        ((ordinal - 24) / 2) % 8
    };
    session.current_word = common::choose_word_common(&session.history, lv);
    db
}

pub async fn state(session: &Session, mut db: Connection<Base>) -> Json<Message> {
    if session.tyv_mode {
        let result_available = session.history.len() >= 60;
        let broad_ito_len = TYV_BROAD.ito.len();
        let question = if (session.current_word as usize) < broad_ito_len {
            &TYV_BROAD.ito[session.current_word as usize]
        } else {
            &TYV_NARROW.ito[session.current_word as usize - broad_ito_len]
        };
        return Json(Message {
            session: 0,
            details: HashMap::from([
                ("result_available".to_string(), result_available.to_string()),
                ("question".to_string(), question.clone()),
            ])
        });
    }
    let result_available = session.history.len() >= 24;
    let row = sqlx::query("SELECT word FROM words WHERE id = ?")
        .bind(session.current_word).fetch_one(&mut **db).await.unwrap();
    let word: String = row.get(0);
    Json(Message {
        session: 0,
        details: HashMap::from([
            ("result_available".to_string(), result_available.to_string()),
            ("question".to_string(), word),
        ])
    })
}

pub async fn submit(
    session: &mut Session, db: BaseConn, data: Json<Message>
) -> (Json<Message>, bool) {
    match data.details["action"].as_str() {
        "choose" => {
            let choice = data.details["recall"] == "true";
            session.history.push((session.current_word, choice));
            let db = update(session, db).await;
            (Json(Message {
                session: 0,
                details: HashMap::new()
            }), false)
        }
        "finish" => {
            if session.history.len() < 24 || (session.tyv_mode && session.history.len() < 60) {
                (Json(Message {
                    session: 0,
                    details: HashMap::from([
                        ("error".to_string(), "Not enough questions answered".to_string())
                    ])
                }), false)
            } else {
                let (details, db) = if session.tyv_mode {
                    (tyv_result(&session.history), db)
                } else {
                    common::result_common(&session.history, db).await
                };
                (Json(Message {
                    session: 0,
                    details,
                }), false)
            }
        }
        _ => panic!("Invalid action"),
    }
}

fn tyv_result(history: &[(u32, bool)]) -> HashMap<String, String> {
    let broad_len = TYV_BROAD.ito.len() as u32;
    let result = history.iter()
        .map(|(i, r)| {
            let word = if *i < broad_len {
                &TYV_BROAD.ito[*i as usize]
            } else {
                &TYV_NARROW.ito[*i as usize - broad_len as usize]
            };
            (&word as &str, *r)
        })
        .collect::<Vec<_>>();
    let est_tyv = vcbe_core::estimate_tyv(&result, &TYV_DATA);
    HashMap::from([
        ("tyv".to_string(), est_tyv.to_string())
    ])
}