use std::path::PathBuf;

use bittorrent_starter_rust::*;

use clap::{Parser, Subcommand};
use tracing::debug;
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
    Info { file: PathBuf },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(layer().without_time())
        .init();

    let args = Args::parse();

    match args.command {
        Commands::Decode { encoded_value } => {
            let decoded_value = Bencode::new(&encoded_value)?;
            let value: serde_json::Value = (&decoded_value).into();
            println!("{}", value);
        }
        Commands::Info { file } => {
            let data = std::fs::read(file)?;
            let torrent: Torrent = serde_bencode::from_bytes(&data)?;
            let Keys::SingleFile { length } = torrent.info.keys;

            debug!("{torrent:#?}");

            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", length);
        }
    }

    Ok(())
}
