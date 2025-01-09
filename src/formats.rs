// use serde::{Deserialize, Serialize};
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub answer: String,
    pub cards: String,

}
