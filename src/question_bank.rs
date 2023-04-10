use std::collections::HashMap;
use std::fs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

pub struct QuestionBank {
    pub questions: HashMap<String, Question>
}

impl Default for QuestionBank {
    fn default() -> Self {
        QuestionBank {
            questions: serde_json::from_str::<HashMap<String, Question>>(
                fs::read_to_string("questions.json").unwrap().as_str()
            ).unwrap()
        }
    }
}

lazy_static! {
    pub static ref QUESTION_BANK: QuestionBank = QuestionBank::default();
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Question {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) description: Option<String>,
    pub(crate) text: String,
    pub(crate) help: Option<String>,
    pub(crate) hints: Vec<Hint>,
}

#[derive(Deserialize, Serialize)]
pub struct Hint {
    pub(crate) title: String,
    pub(crate) description: String,
}