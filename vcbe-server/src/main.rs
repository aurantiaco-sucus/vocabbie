mod standard;

use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use rocket::tokio::sync::{RwLock};
use std::thread;
use std::time::{Duration, Instant};
use log::info;
use once_cell::sync::Lazy;
use rocket::{get, launch, post, routes};
use rocket::http::ContentType;
use rocket::response::content;
use rocket::serde::json::{Json, json, Value};
use rand::random;
use rocket_db_pools::{Connection, Database, sqlx};
use vcbe_core::{Message};

#[launch]
async fn rocket() -> _ {
    rocket::tokio::spawn(async {
        rocket::tokio::time::sleep(Duration::from_secs(2)).await;
        info!("Session watcher thread started.");
        loop {
            rocket::tokio::time::sleep(Duration::from_secs(60)).await;
            info!("Checking for expired sessions.");
            {
                let mut sessions = SESSIONS.write().await;
                let mut untouched = Vec::new();
                for (id, session) in sessions.iter() {
                    if session.read().await.last_access.elapsed().as_secs() > 3000 {
                        untouched.push(*id);
                    }
                }
                for id in untouched {
                    info!("Session {} expired.", id);
                    sessions.remove(&id);
                }
            }
        }
    });
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(1));
        
    });
    rocket::build()
        .attach(Base::init())
        .mount("/", routes![index, start, state, submit])
}

#[get("/")]
fn index() -> (ContentType, &'static str) {
    (ContentType::HTML, include_str!("./index.html"))
}

static SESSIONS: Lazy<RwLock<BTreeMap<u32, Arc<RwLock<Session>>>>> = 
    Lazy::new(|| RwLock::new(BTreeMap::new()));



pub struct Session {
    id: u32,
    last_access: Instant,
    inner: SessionInner
}

impl Deref for Session {
    type Target = SessionInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub enum SessionInner {
    Standard(standard::Session),
}

impl Session {
    pub async fn create_with(inner: SessionInner) -> Arc<RwLock<Self>> {
        let id = {
            let mut id = random();
            while SESSIONS.read().await.contains_key(&id) {
                id = random();
            }
            id
        };
        let session = Arc::new(RwLock::new(Session {
            id,
            last_access: Instant::now(),
            inner,
        }));
        SESSIONS.write().await.insert(id, session.clone());
        session
    }
    
    pub async fn access(id: u32) -> Option<Arc<RwLock<Self>>> {
        SESSIONS.read().await.get(&id).cloned()
    }
}

#[derive(Database)]
#[database("primary")]
pub struct Base(sqlx::MySqlPool);

#[post("/start", format = "json", data = "<data>")]
pub async fn start(data: Json<Message>, db: Connection<Base>) -> Json<Message> {
    match data.details.get("kind").map(|x| x.as_str()) {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(kind) => {
            let session = Session::create_with(match kind {
                "standard" => SessionInner::Standard(standard::create(db).await),
                _ => return Json(Message {
                    session: 0,
                    details: Default::default(),
                })
            }).await;
            let id = session.read().await.id;
            Json(Message {
                session: id,
                details: Default::default(),
            })
        }
    }
}

#[get("/state", format = "json", data = "<data>")]
pub async fn state(data: Json<Message>, db: Connection<Base>) -> Json<Message> {
    match Session::access(data.session).await {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(ses) => {
            let ses = ses.read().await;
            match &ses.inner {
                SessionInner::Standard(ses) => standard::state(ses, db).await,
            }
        }
    }
}

#[post("/submit", format = "json", data = "<data>")]
pub async fn submit(data: Json<Message>, db: Connection<Base>) -> Json<Message> {
    match Session::access(data.session).await {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(ses) => {
            let mut ses = ses.write().await;
            match &mut ses.inner {
                SessionInner::Standard(ses) => standard::submit(ses, db, data).await,
            }
        }
    }
}