use std::collections::HashMap;
use vcbe_core::Message;

fn main() {
    let client = reqwest::blocking::Client::new();
    let resp: Message = client.post("http://localhost:8000/start")
        .json(&Message {
            session: 0,
            details: HashMap::from([
                ("kind".to_string(), "standard".to_string()),
            ]),
        })
        .send().unwrap()
        .json().unwrap();
    let session = resp.session;
    let resp: Message = client.get("http://localhost:8000/state")
        .json(&Message {
            session,
            details: Default::default(),
        })
        .send().unwrap()
        .json().unwrap();
    println!("{:?}", resp);
}
