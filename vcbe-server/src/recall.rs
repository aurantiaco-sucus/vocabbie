use std::collections::{BTreeMap, HashMap};
use once_cell::sync::Lazy;
use rand::{Rng, thread_rng};

use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use vcbe_core::Message;

use crate::{Base, BaseConn, common};

pub struct Session {
    pub history: Vec<(u32, bool)>,
    pub current_word: u32,
    pub tyv_mode: bool,
}

pub async fn create(db: BaseConn, tyv_mode: bool) -> Session {
    let mut session = Session {
        history: Vec::new(),
        current_word: 0,
        tyv_mode
    };
    let _ = update(&mut session, db).await;
    session
}

struct TyvSet {
    ito: Vec<String>,
    toi: BTreeMap<usize, String>,
}

static TYV_BROAD: Lazy<TyvSet> = Lazy::new(|| {
    const RMP: &[u8] = include_bytes!("tyv-broad.rmp");
    let ito: Vec<String> = rmp_serde::from_slice(RMP).unwrap();
    let toi: BTreeMap<usize, String> = ito.iter().enumerate()
        .map(|(i, x)| (i, x.clone()))
        .collect();
    TyvSet { ito, toi }
});

static TYV_NARROW: Lazy<TyvSet> = Lazy::new(|| {
    const RMP: &[u8] = include_bytes!("tyv-narrow.rmp");
    let ito: Vec<String> = rmp_serde::from_slice(RMP).unwrap();
    let toi: BTreeMap<usize, String> = ito.iter().enumerate()
        .map(|(i, x)| (i, x.clone()))
        .collect();
    TyvSet { ito, toi }
});

async fn update(session: &mut Session, db: BaseConn) -> BaseConn {
    let ordinal = session.history.len();
    if session.tyv_mode {
        let range = if ordinal < 40 {
            0..TYV_BROAD.ito.len()
        } else {
            TYV_BROAD.ito.len()..TYV_BROAD.ito.len() + TYV_NARROW.ito.len()
        };
        let mut word = thread_rng().gen_range(&range);
        while session.history.iter().any(|(x, _)| *x == word) {
            word = thread_rng().gen_range(&range);
        }
        session.current_word = word;
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
        let question = if session.current_word as usize < TYV_BROAD.ito.len() {
            &TYV_BROAD.ito[session.current_word]
        } else {
            &TYV_NARROW.ito[session.current_word - TYV_BROAD.ito.len()]
        };
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
            if session.history.len() < 24 {
                (Json(Message {
                    session: 0,
                    details: HashMap::from([
                        ("error".to_string(), "Not enough questions answered".to_string())
                    ])
                }), false)
            } else {
                let (details, db) = common::result_common(
                    &session.history, db).await;
                (Json(Message {
                    session: 0,
                    details,
                }), false)
            }
        }
        _ => panic!("Invalid action"),
    }
}