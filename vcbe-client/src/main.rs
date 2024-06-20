use std::collections::HashMap;
use std::env::args;
use std::io::Write;
use reqwest::blocking::Client;
use vcbe_core::Message;

fn main() {
    let client = Client::new();
    let interface_kind = args().nth(1).unwrap();
    match interface_kind.as_str() {
        "cli" => cli(),
        "script" => script(),
        _ => {}
    }
    let session = std_start(&client);
}

fn cli() {
    let client = Client::new();
    let session = std_start(&client);
    loop {
        let state = std_state(&client, session);
        println!("?\t{}", state.question);
        for (i, candidate) in state.candidates.iter().enumerate() {
            println!("{}\t{}", i, candidate);
        }
        if state.result_available {
            println!("x\t(Finish)");
        }
        print!(">\t");
        std::io::stdout().flush().unwrap();
        let choice = loop {
            let mut choice = String::new();
            std::io::stdin().read_line(&mut choice).unwrap();
            if choice.trim() == "x" && state.result_available {
                break usize::MAX;
            }
            let choice: usize = match choice.trim().parse() {
                Ok(n) => n,
                Err(_) => continue,
            };
            if choice < state.candidates.len() {
                break choice;
            }
        };
        if choice == usize::MAX {
            let result = std_submit(&client, session, StdSubmit::Finish);
            match result {
                StdSubmitResult::Result { uls, rfwls } => {
                    println!("!\tULS: {}, RFWLS: {}", uls, rfwls);
                    break;
                }
                _ => unreachable!(),
            }
        }
        let correct = matches!(
            std_submit(&client, session, StdSubmit::Choose(choice)), 
            StdSubmitResult::Choose(true));
        println!("!\tAnswer {}.", if correct { "correct" } else { "incorrect" });
    }
}

fn script() {
    todo!()
}

fn std_start(client: &Client) -> u32 {
    let resp: Message = client.post("http://localhost:8000/start")
        .json(&Message {
            session: 0,
            details: HashMap::from([
                ("kind".to_string(), "standard".to_string()),
            ]),
        })
        .send().unwrap()
        .json().unwrap();
    assert_ne!(resp.session, 0);
    resp.session
}

struct StdState {
    result_available: bool,
    question: String,
    candidates: Vec<String>,
}

fn std_state(client: &Client, session: u32) -> StdState {
    let resp: Message = client.get("http://localhost:8000/state")
        .json(&Message {
            session,
            details: Default::default(),
        })
        .send().unwrap()
        .json().unwrap();
    let result_available = resp.details["result_available"] == "true";
    let question = resp.details["question"].clone();
    let candidates = resp.details["candidates"].split(";;;")
        .map(|x| x.to_string()).collect();
    StdState { result_available, question, candidates }
}

enum StdSubmit {
    Choose(usize),
    Finish,
}

enum StdSubmitResult {
    Choose(bool),
    NotEnough,
    Result {
        uls: u32,
        rfwls: u32,
    },
}

fn std_submit(client: &Client, session: u32, action: StdSubmit) -> StdSubmitResult {
    let detail = match action {
        StdSubmit::Choose(choice) => HashMap::from([
            ("action".to_string(), "choose".to_string()),
            ("choice".to_string(), choice.to_string()),
        ]),
        StdSubmit::Finish => HashMap::from([
            ("action".to_string(), "finish".to_string()),
        ])
    };
    let resp: Message = client.post("http://localhost:8000/submit")
        .json(&Message {
            session,
            details: detail,
        })
        .send().unwrap()
        .json().unwrap();
    match action {
        StdSubmit::Choose(_) => {
            let correct = resp.details["correct"] == "true";
            StdSubmitResult::Choose(correct)
        }
        StdSubmit::Finish => {
            if resp.details.get("error") == Some(&"not enough questions answered".to_string()) {
                return StdSubmitResult::NotEnough;
            }
            StdSubmitResult::Result {
                uls: resp.details["uls"].parse().unwrap(),
                rfwls: resp.details["rfwls"].parse().unwrap(),
            }
        }
    }
}