mod analysis;
mod app;
mod cli;
mod prompts;
mod tui;

use app::App;
use atrium_api::record::KnownRecord::AppBskyFeedPost;
use clap::Parser;
use cli::Args;
use jetstream_oxide::{
    events::{commit::CommitEvent, JetstreamEvent::Commit},
    DefaultJetstreamEndpoints, JetstreamConfig, JetstreamConnector,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{watch, RwLock};
use tokio::time::interval;
use tui::Tui;
use unicode_segmentation::UnicodeSegmentation;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();

    let args = Args::parse();
    let prompts = prompts::setup_prompts(&args.config)?;

    let data_dir = PathBuf::from(&args.data);
    tokio::fs::create_dir_all(&data_dir).await?;
    let freq_file = data_dir.join("frequencies.json");

    let emoji_freq = Arc::new(RwLock::new(HashMap::new()));
    let freq_clone = emoji_freq.clone();
    let freq_clone_analysis = emoji_freq.clone();

    // Create app instance
    let app = App::new(emoji_freq.clone());

    // Load previous frequencies if they exist
    app.load_frequencies(freq_file.to_str().unwrap()).await?;

    // Handle shutdown signals
    let app_clone = app.clone();
    let app_clone_analysis = app.clone();
    let freq_file_clone = freq_file.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }

        println!("Saving frequency data...");
        if let Err(e) = app_clone
            .save_frequencies(freq_file_clone.to_str().unwrap())
            .await
        {
            eprintln!("Error saving frequencies: {}", e);
        }
        std::process::exit(0);
    });

    // Channel for ops/sec updates
    let (ops_tx, ops_rx) = watch::channel(0);

    // Spawn firehose processing task
    tokio::spawn(async move {
        if let Err(e) = process_firehose(freq_clone, ops_tx).await {
            eprintln!("Firehose error: {}", e);
        }
    });

    // Spawn periodic analysis task
    tokio::spawn({
        let prompts = prompts.clone();
        async move {
            let mut interval = interval(Duration::from_secs(420));
            loop {
                interval.tick().await;
                let freqs = freq_clone_analysis.read().await;

                let prev_analysis = {
                    let prev = app_clone_analysis.current_analysis.read().await;
                    prev.clone()
                };

                {
                    let mut prev = app_clone_analysis.previous_analysis.write().await;
                    *prev = prev_analysis.clone();
                }

                if let Ok(new_analysis) =
                    analysis::analyze_emoji_trends(&freqs, &prev_analysis, &prompts).await
                {
                    let mut current = app_clone_analysis.current_analysis.write().await;
                    *current = new_analysis;
                }
            }
        }
    });

    // Run TUI server
    let tui = Tui::new(ops_rx, args.port).await?;

    tui.run(app).await?;

    Ok(())
}

async fn process_firehose(
    emoji_freq: Arc<RwLock<HashMap<String, usize>>>,
    ops_tx: watch::Sender<usize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = JetstreamConfig {
        endpoint: DefaultJetstreamEndpoints::USEastOne.into(),
        ..Default::default()
    };
    let jetstream = JetstreamConnector::new(config)?;
    let (receiver, _) = jetstream.connect().await?;

    println!("Jetstream listening...");

    let mut ops_count = 0;
    let mut last_check = Instant::now();

    while let Ok(event) = receiver.recv_async().await {
        if let Commit(commit) = event {
            if let CommitEvent::Create { commit, .. } = commit {
                if let AppBskyFeedPost(record) = commit.record {
                    ops_count += 1;

                    // Update ops/sec every second
                    if last_check.elapsed() >= Duration::from_secs(1) {
                        let _ = ops_tx.send(ops_count);
                        ops_count = 0;
                        last_check = Instant::now();
                    }

                    let mut freq = emoji_freq.write().await;
                    for grapheme in record.text.graphemes(true) {
                        if grapheme.chars().any(|c| c.len_utf8() >= 4) {
                            *freq.entry(grapheme.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
