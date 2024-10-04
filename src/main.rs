use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt::layer, prelude::*};

use bittorrent_starter_rust::{bencode::Bencode, message::*, peer::*, torrent::*, Hash};

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
                println!("{}:{}", peer.addr.ip(), peer.addr.port());
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
            let info_hash = torrent.info_hash()?;

            let piece_size = torrent.piece_size(piece);
            let nblocks = piece_size.div_ceil(BLOCK_MAX);

            let mut all_blocks = Vec::with_capacity(piece_size);

            for peer in torrent.peers().await?.iter() {
                let peer = peer.handshake(info_hash).await?;

                let Some(peer) = peer.bitfield(piece).await? else {
                    continue;
                };

                let mut peer = peer.interested().await?;

                for block in 0..nblocks {
                    let block_size = BLOCK_MAX.min(piece_size - BLOCK_MAX * block);

                    let request = Request::new(
                        piece.try_into()?,
                        (block * BLOCK_MAX).try_into()?,
                        block_size.try_into()?,
                    );

                    peer.request(request, &mut all_blocks).await?;
                }

                break;
            }

            let hash = Hash::new(&all_blocks);
            assert_eq!(&*hash, &torrent.info.pieces[piece]);

            tokio::fs::write(&output, all_blocks)
                .await
                .context("write out downloaded piece")?;

            println!("Piece {piece} downloaded to {}", output.display());
        }
    }

    Ok(())
}
