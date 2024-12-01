use hotreload::{Apply, Hotreload};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Clone, Debug)]
pub struct Prompts {
    pub system_prompt: String,
}

#[derive(Default)]
pub struct PromptConfig {
    prompts: Mutex<Prompts>,
}

impl Default for Prompts {
    fn default() -> Self {
        Self {
            system_prompt: "You are an emoji trend analyzer. Provide a brief, one-line insight about the current emoji usage patterns in Bluesky. Be concise, witty, and insightful. Maximum 100 characters.".into(),
        }
    }
}

impl Apply<Prompts> for PromptConfig {
    fn apply(&self, data: Prompts) -> hotreload::ApplyResult {
        let system_prompt = data.system_prompt.clone();
        let mut prompts = self.prompts.lock().unwrap();
        *prompts = data;
        println!("🔄 Prompt configuration reloaded!");
        println!("New system prompt: {}", system_prompt);
        Ok(())
    }
}

pub fn setup_prompts(config_path: &str) -> Result<Arc<PromptConfig>, hotreload::Error> {
    let watcher = Hotreload::<PromptConfig, Prompts>::new(config_path)?;
    Ok(watcher.config().clone())
}

// Add a helper method to get the current prompt
impl PromptConfig {
    pub fn get_system_prompt(&self) -> String {
        self.prompts.lock().unwrap().system_prompt.clone()
    }
}
