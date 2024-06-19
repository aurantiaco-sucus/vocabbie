use std::collections::HashMap;
use rand::{Rng, thread_rng};
use rocket::serde::json::Json;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;
use vcbe_core::Message;
use crate::Base;

const PARTS: [usize; 8] = [1023, 1902, 3595, 6562, 10251, 13612, 12300, 18933];

pub struct Session {
    pub words_id: Vec<u32>,
    pub next_word: u32,
}

pub async fn create(db: Connection<Base>) -> Session {
    Session {
        words_id: Vec::new(),
        next_word: thread_rng().gen_range(0..1023),
    }
}

pub async fn state(session: &Session, mut db: Connection<Base>) -> Json<Message> {
    let ready = session.words_id.len() >= 24;
    let word = sqlx::query(
        "SELECT word FROM words WHERE id = ?")
        .bind(session.next_word)
        .fetch_one(&mut **db).await.unwrap();
    let word: String = word.get(0);
    Json(Message {
        session: 0,
        details: HashMap::from([
            ("ready".to_string(), ready.to_string()),
            ("word".to_string(), word),
        ]),
    })
}

pub async fn submit(session: &mut Session, db: Connection<Base>, data: Json<Message>) -> Json<Message> {
    Json(Message {
        session: 0,
        details: Default::default(),
    })
}