use bittorrent_starter_rust::*;

use clap::{Parser, Subcommand};

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
    let args = Args::parse();

    match args.command {
        Commands::Decode { encoded_value } => {
            let decoded_value = Bencode::new(&encoded_value).unwrap();
            println!("{}", decoded_value.to_value());
        }
    }
}
