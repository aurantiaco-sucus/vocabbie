use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use vcbe_core::Message;

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
    let lv = (ordinal / 100) % 8;
    session.current_words = common::choose_words(&session.history, lv);
    db
}

pub async fn state(session: &Session, mut db: Connection<Base>) -> Json<Message> {
    let result_available = session.history.len() >= 800;
    let mut questions = Vec::new();
    for &word in &session.current_words {
        let row = sqlx::query("SELECT word FROM words WHERE id = ?")
            .bind(word).fetch_one(&mut **db).await.unwrap();
        let word: String = row.get(0);
        questions.push(word);
    }
    Json(Message {
        session: 0,
        details: HashMap::from([
            ("result_available".to_string(), result_available.to_string()),
            ("questions".to_string(), questions.join(";;;")),
        ])
    })
}

pub async fn submit(
    session: &mut Session, db: BaseConn, data: Json<Message>
) -> (Json<Message>, bool) {
    match data.details["action"].as_str() {
        "choose" => {
            let choices = data.details["choices"].split(",")
                .map(|x| x.parse::<bool>().unwrap())
                .collect::<Vec<_>>();
            session.history.extend(session.current_words.iter().copied().zip(choices));
            let db = update(session, db).await;
            (Json(Message {
                session: 0,
                details: HashMap::new()
            }), false)
        }
        "finish" => {
            if session.history.len() < 800 {
                (Json(Message {
                    session: 0,
                    details: HashMap::from([
                        ("error".to_string(), "Not enough questions answered".to_string())
                    ])
                }), false)
            } else {
                let (details, db) = 
                    common::result(&session.history, db).await;
                (Json(Message {
                    session: 0,
                    details,
                }), false)
            }
        }
        _ => panic!("Invalid action"),
    }
}