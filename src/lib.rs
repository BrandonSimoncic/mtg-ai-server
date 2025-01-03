// use std::thread::JoinHandle;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}
pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            println!("Worker {} got a job; executing.", id);
            match message {
                Ok(job)=> {
                    println!("Woker {} got a job; executing.", id);
                    job();
                }
                Err(_) => {
                    println!("Worker {} disconnected. Shutting Down.", id);
                    break;
                }
            }
        });
        Worker { id, thread: Some(thread), }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool { workers, sender: Some(sender)}
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self){
        drop(self.sender.take());
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}


/////////////////////////////////////////////////////////////////////////////////// End of Server Code

use regex::Regex;
use reqwest;
use reqwest::Error;
use reqwest::header::USER_AGENT;

struct Card {
    mtgo_id: String,
    name: String,
    type_line: String,
    oracle_text: String,
    cmc: String,
}

impl Card{
    fn new(card_info: serde_json::Value)->Card{
        let mtgo_id = card_info["mtgo_id"].to_string();
        let name = card_info["name"].to_string();
        let type_line = card_info["type_line"].to_string();
        let oracle_text = card_info["oracle_text"].to_string();
        let cmc = card_info["cmc"].to_string();

        Card { mtgo_id,
            name,
            type_line,
            oracle_text,
            cmc,
        }

    }
}


struct CardsInQuery {
    cards: Vec<Card>,
}


fn find_cards_in_query(query: &str) -> Vec<String> {
    let re = Regex::new(r"\[([^\]]*)\]").unwrap();
    let mut cards: Vec<String> = Vec::new();
    // println!("{}", re.find_iter("Hello, I have 2 apples and 3 oranges").unwrap());
    for cap in re.find_iter(&query){
        
        cards.push(clean_card(cap.as_str().to_string()));
    }
    cards
}
fn clean_card(s: String) -> String{
    s.replace(" ", "+").replace("[", "").replace("]", "")
}

async fn get_scryfall_card(card_name: &str) -> Result<Card, Error> {
    let url = format!("https://api.scryfall.com/cards/named?fuzzy={}", card_name);
    println!("{}", url);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header(USER_AGENT, "AskUgin.com Dev")
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = response.json().await?;
            
    println!("{:#?}", &json["oracle_text"]);

    let card = Card::new(json);

    Ok(card)
}

/////////////////////////////////////////////////////////////////////////////////// Testing Code
/// #[tokio::test]
pub async fn testing(){

    let card = find_cards_in_query("[black lotus] was better than [blood moon]")[0].clone();
    println!("{}", card);
    let text = get_scryfall_card(&card).await;
    // text
}