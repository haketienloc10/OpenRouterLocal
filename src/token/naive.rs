use crate::token::TokenCounter;
use crate::types::normalized::Message;

pub struct NaiveTokenCounter;

impl TokenCounter for NaiveTokenCounter {
    fn count_prompt(&self, _model: &str, messages: &[Message]) -> u64 {
        messages
            .iter()
            .map(|m| m.content.split_whitespace().count() as u64 + 4)
            .sum()
    }

    fn count_completion(&self, _model: &str, text: &str) -> u64 {
        text.split_whitespace().count() as u64
    }
}
