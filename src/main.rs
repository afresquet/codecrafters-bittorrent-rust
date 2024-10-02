use std::{net::SocketAddrV4, path::PathBuf};

use bittorrent_starter_rust::{
    bencode::Bencode,
    peer::Handshake,
    torrent::{Keys, Torrent},
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    Handshake { file: PathBuf, peer: String },
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
        Commands::Handshake { file, peer } => {
            let torrent = Torrent::new(file)?;

            let peer: SocketAddrV4 = peer.parse().context("parse peer address")?;
            let mut peer = tokio::net::TcpStream::connect(peer)
                .await
                .context("connect to peer")?;
            let mut handshake = Handshake::new(torrent.hash()?, *b"00112233445566778899");

            {
                // Safety: Handshake is a POD with repr(C)
                let handshake_bytes =
                    &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];
                let handshake_bytes = unsafe { &mut *handshake_bytes };

                peer.write_all(handshake_bytes)
                    .await
                    .context("write handshake")?;
                peer.read_exact(handshake_bytes)
                    .await
                    .context("read handshake")?;
            }
            assert_eq!(handshake.length, 19);
            assert_eq!(&handshake.bittorrent, b"BitTorrent protocol");

            println!("Peer ID: {}", hex::encode(handshake.peer_id));
        }
    }

    Ok(())
}
