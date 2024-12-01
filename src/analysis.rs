use crate::prompts::PromptConfig;
use async_openai::{
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn analyze_emoji_trends(
    frequencies: &HashMap<String, usize>,
    previous_analysis: &str,
    prompts: &Arc<PromptConfig>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();

    let data_str = frequencies
        .iter()
        .take(100)
        .map(|(emoji, count)| format!("{}: {}", emoji, count))
        .collect::<Vec<_>>()
        .join(", ");

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(100u16)
        .model("gpt-4o-mini")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(prompts.get_system_prompt())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!(
                    "Previous analysis: '{}'. Here are the current top emoji frequencies: {}. What's interesting about this?",
                    previous_analysis, data_str
                ))
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    let analysis = response.choices[0]
        .message
        .content
        .clone()
        .unwrap_or_default();

    Ok(analysis)
}
