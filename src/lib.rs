use regex::Regex;
use reqwest;
use reqwest::Error;
use reqwest::header::USER_AGENT;
use serde_json::json;
use std::env;
use dotenv::dotenv;
struct Card {
    mtgo_id: String,
    name: String,
    type_line: String,
    oracle_text: String,
    cmc: String,
    rulings: Vec<String>,
}

impl Card{
    async fn new(card_info: serde_json::Value)->Card{
        let mtgo_id = card_info["mtgo_id"].to_string();
        let name = card_info["name"].to_string();
        let type_line = card_info["type_line"].to_string();
        let oracle_text = card_info["oracle_text"].to_string();
        let cmc = card_info["cmc"].to_string();
        let rulings = get_ruling_uri(&card_info["rulings_uri"]
        .to_string())
        .await
        .unwrap();

        Card { mtgo_id,
            name,
            type_line,
            oracle_text,
            cmc,
            rulings,
        }

    }

    async fn dump(card: &Card) -> String{
     format!("Card Name: {}
            Card Text: {}
            Card Typing: {}
            Converted Mana Cost: {}
            Rulings: {}",
            card.name,
            card.oracle_text,
            card.type_line,
            card.cmc,
            card.rulings.join(" "))
    }
}

fn find_cards_in_query(query: &str) -> Vec<String> {
    let re = Regex::new(r"\[([^\]]*)\]").unwrap();
    let mut cards: Vec<String> = Vec::new();
    for cap in re.find_iter(&query){
        
        cards.push(clean_card(cap.as_str().to_string()));
    }
    cards
}
fn clean_card(s: String) -> String{
    s.replace(" ", "+")
    .replace("[", "")
    .replace("]", "")
}

async fn get_scryfall_card(card_name: &str) -> Result<Card, Error> {
    let url = format!("https://api.scryfall.com/cards/named?fuzzy={}", card_name);
    
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;
    let response = client
        .get(&url)
        .header(USER_AGENT, "AskUgin.com/1.0")
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    let card = Card::new(json).await;

    Ok(card)
}

async fn get_ruling_uri(url: &str) -> Result<Vec<String>, Error> {
    // let url = format!(&url);
    let re = Regex::new(r#""(https://api\.scryfall\.com/cards/[a-z0-9-]+/rulings)""#).unwrap();
    let caps = re.captures(url).unwrap();
    let extracted_url = caps.get(1).map_or("", |m| m.as_str());
    
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;
    let response = client
        .get(extracted_url)
        .header(USER_AGENT, "AskUgin.com/1.0")
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    let mut comments = Vec::new();
    if let Some(data_array) = json["data"].as_array() {
        for item in data_array {
            if let Some(comment) = item["comment"].as_str() {
                comments.push(comment.to_string());
            }
        }
    }
    Ok(comments)
}

async fn get_openrouter_key()-> String{
    dotenv().ok();
    let api_key = env::var("OPENROUTER_API");
    match api_key {
        Ok(val) => {
            val
        }, 
        Err(e) => {format!("Error API_KEY: {}", e)},
    }
}
pub async fn get_askugin_key()-> String{
    dotenv().ok();
    let api_key = env::var("ASKUGIN");
    match api_key {
        Ok(val) => {
            val
        }, 
        Err(e) => {format!("Error API_KEY: {}", e)},
    }
}


async fn clean_ugin_answer(answer: String) -> String{
    answer.replace("\\n\\n", "  \\n")
        .replace("\\n", "  \\n")
        .replace("\\\\n", "  \\n")
    
}

async fn get_ai_ruling(query: &str) ->Result<String, Error>{
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;
    let url = "https://openrouter.ai/api/v1/chat/completions";
    let api_key = get_openrouter_key().await;
    let site_url = "askugin.com";
    let site_name = "Ask Ugin";
    let body = json!({
        "model": "moonshotai/kimi-k2:free",
        "messages": [
            {
                "role": "user",
                "content": query 
            }
        ]
    });

    let response = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", site_url)
        .header("X-Title", site_name)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let answer = json["choices"][0]["message"]["content"].to_string();
    let answer = clean_ugin_answer(answer).await;

    Ok(answer)
}

fn construct_query(query: &str, name_of_cards: &str , cards: &str) -> String{

    let query = format!("
    You are a Magic: The Gathering Judge.
    Explain how {} interact in a hypothetical scenario based on the following information:
    
    User Question: {}
    Card Details: {}
    Reasoning Requirements:
            Think about each card's abilities and effects separately.
            Address the specific user question using these abilities and rulings.
            If there is a colon, before the colon is a cost, after the colon is an effect.
            Explain how timing, priority, and state-based actions affect the interaction, if applicable.

    Be clear, concise, and ensure your explanation aligns with the rules of Magic: The Gathering. 
    Assume the user has a basic understanding of the game mechanics but may not grasp complex rulings.
    Do not reference cards outside of the question. Do not make examples that are outside of the given cards.
    Please only respond in plain text format
    Please only place your context within the <think> tags.
    ",
    name_of_cards, 
    query, 
    cards);

    query
}

pub async fn ask_ugin(query: &str) -> (String, String) {
    let cards = find_cards_in_query(&query);
    let cards_with_info: Vec<_> = futures::future::join_all(
        cards.iter().map(|card| get_scryfall_card(card))
    ).await;
    let mut card_dump:String = String::new();
    let mut name_of_cards:String = String::new();


    for cards in cards_with_info{
        if let Ok(card) = cards{
            let card_text = Card::dump(&card).await;

            name_of_cards += "  \\n ";
            name_of_cards += &card.name;

            // card_dump += "  \\n ";
            if card_dump == ""{
                card_dump = format!("{}", card_text);
            }
            else {
                card_dump = format!("{}
                {}", card_dump, card_text);
            }
            
            
        }
        else {
            println!("We got got.")
        }
    }
    let query = construct_query(query, &name_of_cards, &card_dump);
    let judge_one = get_ai_ruling(&query).await;
    // let judge_two = get_ai_ruling(&query, &card_dump).await;
    // let judge_three = get_ai_ruling(&query, &card_dump).await;
    // let head_judge = get_ai_ruling(&query, &card_dump).await;
    
    let answer = judge_one.unwrap();

    let pat = Regex::new(r"(?ims)[`\s]*[\[<]think[>\]]](.*?)[\[<]/think[>\]]][`\s]*|^[`\s]*([\[<]thinking[>\]]][`\s]*.*)$").unwrap();
    let answer_cleaned = pat
        .captures(&answer)
        .and_then(|cap| cap.get(1).or_else(|| cap.get(2)))
        .map(|m| m.as_str().to_string())
        .unwrap_or(answer);
    (answer_cleaned, card_dump)
}

//how does [urza's saga] work when [blood moon] is played?