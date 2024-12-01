use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the prompts configuration file
    #[arg(short, long, default_value = "config/prompts.toml")]
    pub config: String,
    #[arg(short, long, default_value = "/var/lib/emojiopticon")]
    pub data: String,
    #[arg(short, long, default_value_t = 1337)]
    pub port: u16,
}
