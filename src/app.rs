use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Default)]
pub struct FrequencyData {
    pub emoji_counts: HashMap<String, usize>,
}

#[derive(Clone)]
pub struct App {
    pub emoji_freq: Arc<RwLock<HashMap<String, usize>>>,
    pub current_analysis: Arc<RwLock<String>>,
    pub previous_analysis: Arc<RwLock<String>>,
    pub start_time: Arc<Instant>,
}

impl App {
    pub fn new(emoji_freq: Arc<RwLock<HashMap<String, usize>>>) -> Self {
        Self {
            emoji_freq,
            current_analysis: Arc::new(RwLock::new(String::from("Analyzing emoji trends..."))),
            previous_analysis: Arc::new(RwLock::new(String::from("No previous analysis."))),
            start_time: Arc::new(Instant::now()),
        }
    }

    pub async fn save_frequencies(
        &self,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let freqs = self.emoji_freq.read().await;
        let data = FrequencyData {
            emoji_counts: freqs.clone(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    pub async fn load_frequencies(
        &self,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(json) = tokio::fs::read_to_string(path).await {
            if let Ok(data) = serde_json::from_str::<FrequencyData>(&json) {
                let mut freqs = self.emoji_freq.write().await;
                *freqs = data.emoji_counts;
            }
        }
        Ok(())
    }
}
