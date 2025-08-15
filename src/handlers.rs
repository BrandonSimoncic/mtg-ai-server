use mtg_ai_server::ask_ugin;
use super::formats::Post;
use std::collections::HashMap;
use mtg_ai_server::get_askugin_key;

// A function to handle GET requests at /posts/{id}
pub async fn get_post(
    id: String,
    authorization: String,
    referer: String,
    title: String,
    query_params: HashMap<String, String>,) -> Result<impl warp::Reply, warp::Rejection> {
    if authorization == get_askugin_key().await {
        let (answer, cards) = ask_ugin(&query_params["question"].to_string()).await;
        let post = Post {
            id,
            title: String::from("Hello, The Server!"),
            answer: answer,
            cards: cards,
        };
        Ok(warp::reply::json(&post))
}else {
        
        let post = Post {
            id,
            title: String::from("Hello, The Server!"),
            answer: String::from("Waiting for Ugin..."),
            cards: String::from("Waiting for Ugin..."),
        };
        Ok(warp::reply::json(&post))
    }
}
