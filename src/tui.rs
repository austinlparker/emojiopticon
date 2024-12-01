use crate::app::App;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};

#[derive(Clone)]
struct DisplayUpdate {
    content: String,
}

#[derive(Clone)]
struct Metrics {
    connected_clients: Arc<AtomicUsize>,
    ops_per_sec: watch::Receiver<usize>,
}

pub struct Tui {
    listener: TcpListener,
    tx: broadcast::Sender<DisplayUpdate>,
    metrics: Metrics,
}

impl Tui {
    pub async fn new(
        ops_rx: watch::Receiver<usize>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind("0.0.0.0:3000").await?;
        let (tx, _) = broadcast::channel(16);

        let metrics = Metrics {
            connected_clients: Arc::new(AtomicUsize::new(0)),
            ops_per_sec: ops_rx,
        };

        println!("Server listening on port 3000");
        Ok(Self {
            listener,
            tx,
            metrics,
        })
    }

    pub async fn run(self, app: App) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tx = self.tx.clone();
        let app = Arc::new(app);
        let metrics = self.metrics;

        // Spawn display update task
        {
            let tx = tx.clone();
            let app = app.clone();
            let metrics = metrics.clone();

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;

                    // Scope the locks to ensure they're dropped
                    let content = {
                        let freqs = app.emoji_freq.read().await;
                        let current = app.current_analysis.read().await;

                        format_display(
                            &freqs,
                            metrics.connected_clients.load(Ordering::Relaxed),
                            *metrics.ops_per_sec.borrow(),
                            &current,
                            &app.start_time,
                        )
                    };

                    let _ = tx.send(DisplayUpdate { content });
                }
            });
        }

        // Handle incoming connections
        while let Ok((mut socket, addr)) = self.listener.accept().await {
            println!("New connection from: {}", addr);
            let mut rx = tx.subscribe();
            let clients = metrics.connected_clients.clone();
            clients.fetch_add(1, Ordering::Relaxed);

            tokio::spawn(async move {
                let _ = socket.write_all(b"\x1b[?1049h\x1b[?25l").await;

                while let Ok(update) = rx.recv().await {
                    if socket.write_all(update.content.as_bytes()).await.is_err() {
                        break;
                    }
                }

                clients.fetch_sub(1, Ordering::Relaxed);
                let _ = socket.write_all(b"\x1b[?25h\x1b[?1049l").await;
            });
        }

        Ok(())
    }
}

fn create_gradient_bar(count: usize) -> String {
    let width = count.min(50);
    let log_count = (count as f64 + 1.0).log10();

    let gradients = [
        ("\x1b[38;5;46m█\x1b[0m", 1),    // bright green
        ("\x1b[38;5;82m█\x1b[0m", 10),   // light green
        ("\x1b[38;5;226m█\x1b[0m", 50),  // yellow
        ("\x1b[38;5;214m█\x1b[0m", 100), // orange
        ("\x1b[38;5;196m█\x1b[0m", 500), // red
    ];

    let mut bar = String::new();
    for i in 0..width {
        let position = i as f64 / width as f64;
        let gradient_index = ((position * log_count * 1.5) as usize).min(gradients.len() - 1);
        bar.push_str(gradients[gradient_index].0);
    }
    bar
}

fn format_display(
    freqs: &HashMap<String, usize>,
    connected_clients: usize,
    ops_per_sec: usize,
    current_analysis: &str,
    start_time: &Instant,
) -> String {
    let mut output = String::new();

    // Clear screen and move to top
    output.push_str("\x1B[2J\x1B[H");

    // Fancy header
    output.push_str(&format!(
        "{}",
        r"
 ███████╗███╗   ███╗ ██████╗      ██╗██╗ ██████╗ ██████╗ ████████╗██╗ ██████╗ ██████╗ ███╗   ██╗
 ██╔════╝████╗ ████║██╔═══██╗     ██║██║██╔═══██╗██╔══██╗╚══██╔══╝██║██╔════╝██╔═══██╗████╗  ██║
 █████╗  ██╔████╔██║██║   ██║     ██║██║██║   ██║██████╔╝   ██║   ██║██║     ██║   ██║██╔██╗ ██║
 ██╔══╝  ██║╚██╔╝██║██║   ██║██   ██║██║██║   ██║██╔═══╝    ██║   ██║██║     ██║   ██║██║╚██╗██║
 ███████╗██║ ╚═╝ ██║╚██████╔╝╚█████╔╝██║╚██████╔╝██║        ██║   ██║╚██████╗╚██████╔╝██║ ╚████║
 ╚══════╝╚═╝     ╚═╝ ╚═════╝  ╚════╝ ╚═╝ ╚═════╝ ╚═╝        ╚═╝   ╚═╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═══╝
"
    ));

    output.push_str(&"─".repeat(100));
    output.push_str("\n\n");

    // Sort and display frequencies
    let mut freq_vec: Vec<_> = freqs.iter().collect();
    freq_vec.sort_by(|a, b| b.1.cmp(a.1));
    freq_vec.truncate(25);

    for (emoji, count) in freq_vec {
        let bar = create_gradient_bar(*count);
        output.push_str(&format!("{:4} {:5} {}\n", emoji, count, bar));
    }

    output.push_str("\n");
    output.push_str(&"─".repeat(100));
    output.push_str(&format!("\n📊 Analysis: {}\n", current_analysis));

    let uptime = start_time.elapsed();
    let days = uptime.as_secs() / 86400;
    let hours = (uptime.as_secs() % 86400) / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    let uptime_str = if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    };

    output.push_str(&"─".repeat(100));
    output.push_str(&format!(
        "\n🚀 by @aparker.io | connected: {} | posts/sec: {} | uptime: {} | press ctrl+c to disconnect.\n",
        connected_clients, ops_per_sec, uptime_str
    ));

    output
}
