use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use vcbe_core::Message;

use crate::{Base, BaseConn};

pub struct Session {
    pub history: Vec<(u32, bool)>,
    pub current_word: u32,
}

pub async fn create(db: BaseConn) -> Session {
    let mut session = Session {
        history: Vec::new(),
        current_word: 0,
    };
    let _ = update(&mut session, db).await;
    session
}

async fn update(session: &mut Session, db: BaseConn) -> BaseConn {
    let ordinal = session.history.len();
    let lv = if ordinal < 24 {
        ordinal / 3
    } else {
        ((ordinal - 24) / 2) % 8
    };
    session.current_word = super::standard::choose_word_common(&session.history, lv);
    db
}

pub async fn state(session: &Session, mut db: Connection<Base>) -> Json<Message> {
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
                let (details, db) = super::standard::result_common(
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