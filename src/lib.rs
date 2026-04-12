use regex::Regex;
use reqwest::Error;
use reqwest::header::USER_AGENT;
use serde_json::json;
use std::env;
use std::sync::LazyLock;
use dotenv::dotenv;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder().use_rustls_tls().build().unwrap()
});

static OPENROUTER_KEY: LazyLock<String> = LazyLock::new(|| {
    dotenv().ok();
    env::var("OPENROUTER_API").unwrap_or_default()
});

static ASKUGIN_KEY: LazyLock<String> = LazyLock::new(|| {
    dotenv().ok();
    env::var("ASKUGIN").unwrap_or_default()
});

struct Card {
    name: String,
    type_line: String,
    oracle_text: String,
    cmc: String,
    rulings: Vec<String>,
}

impl Card {
    fn dump(&self) -> String {
        format!(
            "Card Name: {}
            Card Text: {}
            Card Typing: {}
            Converted Mana Cost: {}
            Rulings: {}",
            self.name,
            self.oracle_text,
            self.type_line,
            self.cmc,
            self.rulings.join(" ")
        )
    }
}

fn find_cards_in_query(query: &str) -> Vec<String> {
    let re = Regex::new(r"\[([^\]]*)\]").unwrap();
    re.find_iter(query)
        .map(|cap| clean_card(cap.as_str().to_string()))
        .collect()
}

fn clean_card(s: String) -> String {
    s.replace(" ", "+")
        .replace("[", "")
        .replace("]", "")
}

async fn get_scryfall_card(card_name: &str) -> Result<Card, Error> {
    let url = format!("https://api.scryfall.com/cards/named?fuzzy={}", card_name);

    let response = HTTP_CLIENT.get(&url)
        .header(USER_AGENT, "AskUgin.com/1.0")
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    let rulings_uri = json["rulings_uri"].as_str().unwrap_or("").to_string();
    let rulings = get_ruling_uri(&rulings_uri).await.unwrap_or_default();

    Ok(Card {
        name: json["name"].as_str().unwrap_or("").to_string(),
        type_line: json["type_line"].as_str().unwrap_or("").to_string(),
        oracle_text: json["oracle_text"].as_str().unwrap_or("").to_string(),
        cmc: json["cmc"].to_string(),
        rulings,
    })
}

async fn get_ruling_uri(url: &str) -> Result<Vec<String>, Error> {
    let response = HTTP_CLIENT.get(url)
        .header(USER_AGENT, "AskUgin.com/1.0")
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;

    let mut comments = Vec::new();
    if let Some(data_array) = json["data"].as_array() {
        // Take the 5 most recent rulings to keep the prompt concise
        for item in data_array.iter().rev().take(5) {
            if let Some(comment) = item["comment"].as_str() {
                comments.push(comment.to_string());
            }
        }
        comments.reverse();
    }
    Ok(comments)
}

pub fn get_askugin_key() -> &'static str {
    &ASKUGIN_KEY
}

fn clean_ugin_answer(answer: String) -> String {
    answer.replace("\\n\\n", "  \\n")
        .replace("\\n", "  \\n")
        .replace("\\\\n", "  \\n")
}

async fn get_ai_ruling(query: &str) -> Result<String, Error> {
    let body = json!({
        "model": "nvidia/nemotron-3-nano-30b-a3b:free",
        "messages": [
            {
                "role": "user",
                "content": query
            }
        ]
    });

    let response = HTTP_CLIENT.post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", *OPENROUTER_KEY))
        .header("HTTP-Referer", "askugin.com")
        .header("X-Title", "Ask Ugin")
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let answer = json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
    Ok(clean_ugin_answer(answer))
}

fn construct_query(query: &str, name_of_cards: &str, cards: &str) -> String {
    format!("
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
    Please only respond in plain text format.
    Please only place your reasoning within the <think> tags.
    Structure your response exactly as follows:
    SHORT_ANSWER: (1-2 sentences directly answering the question)
    DETAILED_ANSWER: (full explanation of the ruling and interaction)
    ",
    name_of_cards,
    query,
    cards)
}

pub async fn ask_ugin(query: &str) -> (String, String, String) {
    let card_names = find_cards_in_query(query);
    let cards_with_info: Vec<_> = futures::future::join_all(
        card_names.iter().map(|card| get_scryfall_card(card))
    ).await;

    let mut card_dump = String::new();
    let mut name_of_cards = String::new();

    for result in cards_with_info {
        if let Ok(card) = result {
            let card_text = card.dump();
            name_of_cards += "  \\n ";
            name_of_cards += &card.name;

            if card_dump.is_empty() {
                card_dump = card_text;
            } else {
                card_dump = format!("{}\n                {}", card_dump, card_text);
            }
        } else {
            println!("We got got.")
        }
    }

    let query = construct_query(query, &name_of_cards, &card_dump);
    let answer = get_ai_ruling(&query).await.unwrap();

    let think_pat = Regex::new(r"(?ims)[`\s]*[\[<]think[>\]]](.*?)[\[<]/think[>\]]][`\s]*|^[`\s]*([\[<]thinking[>\]]][`\s]*.*)$").unwrap();
    let answer_cleaned = think_pat
        .captures(&answer)
        .and_then(|cap| cap.get(1).or_else(|| cap.get(2)))
        .map(|m| m.as_str().to_string())
        .unwrap_or(answer);

    let short_pat = Regex::new(r"(?i)SHORT_ANSWER:\s*(.+?)(?=DETAILED_ANSWER:|$)").unwrap();
    let long_pat = Regex::new(r"(?is)DETAILED_ANSWER:\s*(.+)").unwrap();

    let short_answer = short_pat
        .captures(&answer_cleaned)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let long_answer = long_pat
        .captures(&answer_cleaned)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or(answer_cleaned);

    (short_answer, long_answer, card_dump)
}

//how does [urza's saga] work when [blood moon] is played?
