use crate::types::normalized::Message;

pub mod naive;

pub trait TokenCounter: Send + Sync {
    fn count_prompt(&self, model: &str, messages: &[Message]) -> u64;
    fn count_completion(&self, model: &str, text: &str) -> u64;
}
