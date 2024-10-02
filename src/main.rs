use std::path::PathBuf;

use bittorrent_starter_rust::{
    bencode::Bencode,
    torrent::{Keys, Torrent},
};

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
    Info { file: PathBuf },
    Peers { file: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            let torrent = Torrent::new(file)?;
            let Keys::SingleFile { length } = torrent.info.keys;
            let info_hash = hex::encode(torrent.hash()?);

            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", length);
            println!("Info Hash: {}", info_hash);
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            for piece_hash in torrent.piece_hashes() {
                println!("{piece_hash}");
            }
        }
        Commands::Peers { file } => {
            let torrent = Torrent::new(file)?;

            for peer in torrent.peers().await?.peers.iter() {
                println!("{}:{}", peer.ip(), peer.port());
            }
        }
    }

    Ok(())
}
