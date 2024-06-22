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
    match args().nth(2).unwrap().as_str() {
        "std" => cli_std(),
        "rcl" => cli_rcl(),
        _ => {}
    }
}

fn cli_std() {
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

fn cli_rcl() {
    let client = Client::new();
    let session = rcl_start(&client);
    loop {
        let state = rcl_state(&client, session);
        println!("?\t{}", state.question);
        println!("y\tYes");
        println!("n\tNo");
        if state.result_available {
            println!("x\tFinish");
        }
        print!(">\t");
        std::io::stdout().flush().unwrap();
        let choice = loop {
            let mut choice = String::new();
            std::io::stdin().read_line(&mut choice).unwrap();
            match choice.trim() {
                "y" => break 0,
                "n" => break 1,
                "x" if state.result_available => break 2,
                _ => continue,
            }
        };
        match choice {
            0 => {
                let _ = rcl_submit(&client, session, RclSubmit::Choose(true));
            }
            1 => {
                let _ = rcl_submit(&client, session, RclSubmit::Choose(false));
            }
            2 => {
                let result = rcl_submit(&client, session, RclSubmit::Finish);
                match result {
                    RclSubmitResult::Result { uls, rfwls } => {
                        println!("!\tULS: {}, RFWLS: {}", uls, rfwls);
                        break;
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}

fn script() {
    let file = args().nth(2).unwrap();
    let script = std::fs::read_to_string(file).unwrap();
    let client = Client::new();
    let mut lines = script.lines()
        .filter(|x| !x.is_empty());
    let mode = lines.next().unwrap().trim();
    match mode {
        "std" => {
            let session = std_start(&client);
            let mut last = StdSubmitResult::NotEnough;
            for line in lines {
                let choice = line.parse().unwrap();
                last = std_submit(&client, session, StdSubmit::Choose(choice));
            }
            if matches!(last, StdSubmitResult::NotEnough) {
                eprintln!("Not enough questions answered.");
            }
            last = std_submit(&client, session, StdSubmit::Finish);
            if let StdSubmitResult::Result { uls, rfwls } = last {
                println!("ULS: {}, RFWLS: {}", uls, rfwls);
            } else {
                eprintln!("Unexpected result.");
            }
        }
        "rcl" => {
            let session = rcl_start(&client);
            let mut last = RclSubmitResult::NotEnough;
            for line in lines {
                let choice = match line.trim() {
                    "y" => RclSubmit::Choose(true),
                    "n" => RclSubmit::Choose(false),
                    _ => unreachable!(),
                };
                last = rcl_submit(&client, session, choice);
            }
            last = rcl_submit(&client, session, RclSubmit::Finish);
            if matches!(last, RclSubmitResult::NotEnough) {
                eprintln!("Not enough questions answered.");
            }
            if let RclSubmitResult::Result { uls, rfwls } = last {
                println!("ULS: {}, RFWLS: {}", uls, rfwls);
            } else {
                eprintln!("Unexpected result.");
            }
        }
        _ => unreachable!(),
    };
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

fn rcl_start(client: &Client) -> u32 {
    let resp: Message = client.post("http://localhost:8000/start")
        .json(&Message {
            session: 0,
            details: HashMap::from([
                ("kind".to_string(), "recall".to_string()),
            ]),
        })
        .send().unwrap()
        .json().unwrap();
    assert_ne!(resp.session, 0);
    resp.session
}

struct RclState {
    result_available: bool,
    question: String
}

fn rcl_state(client: &Client, session: u32) -> RclState {
    let resp: Message = client.get("http://localhost:8000/state")
        .json(&Message {
            session,
            details: Default::default(),
        })
        .send().unwrap()
        .json().unwrap();
    let result_available = resp.details["result_available"] == "true";
    let question = resp.details["question"].clone();
    RclState { result_available, question }
}

enum RclSubmit {
    Choose(bool),
    Finish,
}

enum RclSubmitResult {
    Choose,
    NotEnough,
    Result {
        uls: u32,
        rfwls: u32,
    },
}

fn rcl_submit(client: &Client, session: u32, action: RclSubmit) -> RclSubmitResult {
    let detail = match action {
        RclSubmit::Choose(recall) => HashMap::from([
            ("action".to_string(), "choose".to_string()),
            ("recall".to_string(), recall.to_string()),
        ]),
        RclSubmit::Finish => HashMap::from([
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
        RclSubmit::Choose(_) => RclSubmitResult::Choose,
        RclSubmit::Finish => {
            if resp.details.get("error") == Some(&"not enough questions answered".to_string()) {
                return RclSubmitResult::NotEnough;
            }
            println!("{:?}", resp);
            RclSubmitResult::Result {
                uls: resp.details["uls"].parse().unwrap(),
                rfwls: resp.details["rfwls"].parse().unwrap(),
            }
        }
    }
}
