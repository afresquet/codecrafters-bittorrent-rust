use bittorrent_starter_rust::*;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt::layer, prelude::*};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Decode { encoded_value: String },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    tracing_subscriber::registry()
        .with(layer().without_time())
        .init();

    let args = Args::parse();

    match args.command {
        Commands::Decode { encoded_value } => {
            let decoded_value = Bencode::new(&encoded_value).unwrap();
            let value: serde_json::Value = (&decoded_value).into();
            println!("{}", value);
        }
    }
}
