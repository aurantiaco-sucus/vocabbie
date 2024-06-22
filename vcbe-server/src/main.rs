mod standard;
mod recall;
mod common;

use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use rocket::tokio::sync::{RwLock};
use std::thread;
use std::time::{Duration, Instant};
use log::{info, warn};
use once_cell::sync::Lazy;
use rocket::{get, launch, options, post, Request, Response, routes};
use rocket::http::{ContentType, Header};
use rocket::serde::json::{Json};
use rand::random;
use rocket::fairing::{Fairing, Info, Kind};
use rocket_db_pools::{Connection, Database, sqlx};
use vcbe_core::{Message};

#[launch]
async fn rocket() -> _ {
    rocket::tokio::spawn(async {
        rocket::tokio::time::sleep(Duration::from_secs(2)).await;
        info!("Session watcher thread started.");
        #[cfg(feature = "permissive")] {
            warn!("Permissive feature is enabled.");
        }
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
        .attach(CORS)
        .mount("/", routes![
            index, start, start_options, state, state_options, state_post, submit, submit_options
        ])
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Permissive CORS handling",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", 
                                        "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", 
                                        "true"));
    }
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
    Recall(recall::Session),
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
    
    pub async fn terminate(id: u32) {
        SESSIONS.write().await.remove(&id);
    }
}

#[derive(Database)]
#[database("primary")]
pub struct Base(sqlx::MySqlPool);

pub type BaseConn = Connection<Base>;
pub type WithConn<T> = (T, BaseConn);

#[post("/start", format = "json", data = "<data>")]
pub async fn start(data: Json<Message>, db: BaseConn) -> Json<Message> {
    match data.details.get("kind").map(|x| x.as_str()) {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(kind) => {
            let session = Session::create_with(match kind {
                "standard" => SessionInner::Standard(standard::create(db).await),
                "recall" => SessionInner::Recall(recall::create(db, false).await),
                "recall-tyv" => SessionInner::Recall(recall::create(db, true).await),
                _ => return Json(Message {
                    session: 0,
                    details: HashMap::from([
                        ("error".to_string(), "invalid session kind".to_string())
                    ])
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

#[options("/start")]
pub async fn start_options() { }

#[get("/state", format = "json", data = "<data>")]
pub async fn state(data: Json<Message>, db: BaseConn) -> Json<Message> {
    match Session::access(data.session).await {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(ses) => {
            let ses = ses.read().await;
            match &ses.inner {
                SessionInner::Standard(ses) => standard::state(ses, db).await,
                SessionInner::Recall(ses) => recall::state(ses, db).await,
            }
        }
    }
}

#[post("/state", format = "json", data = "<data>")]
pub async fn state_post(data: Json<Message>, db: BaseConn) -> Json<Message> {
    state(data, db).await
}

#[options("/state")]
pub async fn state_options() { }

#[post("/submit", format = "json", data = "<data>")]
pub async fn submit(data: Json<Message>, db: BaseConn) -> Json<Message> {
    let sid = data.session;
    match Session::access(sid).await {
        None => Json(Message {
            session: 0,
            details: Default::default(),
        }),
        Some(ses) => {
            let (resp, term) = {
                let mut ses = ses.write().await;
                match &mut ses.inner {
                    SessionInner::Standard(ses) => 
                        standard::submit(ses, db, data).await,
                    SessionInner::Recall(ses) =>
                        recall::submit(ses, db, data).await,
                }
            };
            if term {
                Session::terminate(sid).await;
            }
            resp
        }
    }
}

#[options("/submit")]
pub async fn submit_options() { }