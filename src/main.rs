use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt::layer, prelude::*};

use bittorrent_starter_rust::{bencode::Bencode, peer::*, torrent::*};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[clap(rename_all = "snake_case")]
enum Commands {
    Decode {
        encoded_value: String,
    },
    Info {
        torrent: PathBuf,
    },
    Peers {
        torrent: PathBuf,
    },
    Handshake {
        torrent: PathBuf,
        peer: String,
    },
    DownloadPiece {
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
        piece: usize,
    },
    Download {
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
    },
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
        Commands::Info { torrent } => {
            let torrent = Torrent::new(torrent).await?;
            let Keys::SingleFile { length } = torrent.info.keys;
            let info_hash = hex::encode(torrent.info_hash()?);

            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", length);
            println!("Info Hash: {}", info_hash);
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            for piece_hash in torrent.piece_hashes() {
                println!("{piece_hash}");
            }
        }
        Commands::Peers { torrent } => {
            let torrent = Torrent::new(torrent).await?;

            for peer in torrent.peers().await?.iter() {
                println!("{}:{}", peer.addr().ip(), peer.addr().port());
            }
        }
        Commands::Handshake { torrent, peer } => {
            let torrent = Torrent::new(torrent).await?;

            let peer = Peer::try_from(peer)?
                .handshake(torrent.info_hash()?)
                .await?;

            println!("Peer ID: {}", hex::encode(peer.id()));
        }
        Commands::DownloadPiece {
            output,
            torrent,
            piece,
        } => {
            let torrent = Torrent::new(torrent).await?;
            let data = torrent.download_pieces(std::iter::once(piece)).await?;

            tokio::fs::write(&output, data)
                .await
                .context("write out downloaded piece")?;

            println!("Piece {piece} downloaded to {}", output.display());
        }
        Commands::Download { output, torrent } => {
            let torrent = Torrent::new(torrent).await?;
            let data = torrent.download().await?;

            tokio::fs::write(&output, data)
                .await
                .context("write out downloaded file")?;

            println!("File downloaded to {}", output.display());
        }
    }

    Ok(())
}
