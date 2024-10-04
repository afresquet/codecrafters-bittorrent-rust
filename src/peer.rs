use std::{marker::PhantomData, net::SocketAddrV4};

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::Framed;

use crate::{message::*, torrent::BLOCK_MAX};

pub struct Uninitialized;
pub struct Handshook;
pub struct Bitfield;
pub struct Ready;

pub struct Peer<H> {
    pub addr: SocketAddrV4,
    id: Option<[u8; 20]>,
    session: Option<Framed<TcpStream, MessageFramer>>,
    handshake: PhantomData<H>,
}

impl Peer<Uninitialized> {
    pub fn new(addr: SocketAddrV4) -> Self {
        Self {
            addr,
            id: None,
            session: None,
            handshake: PhantomData,
        }
    }

    pub async fn handshake(self, info_hash: [u8; 20]) -> anyhow::Result<Peer<Handshook>> {
        let mut stream = tokio::net::TcpStream::connect(self.addr).await?;
        let mut handshake = Handshake::new(info_hash);
        let bytes = handshake.as_bytes_mut();
        stream.write_all(bytes).await?;
        stream.read_exact(bytes).await?;
        assert_eq!(handshake.length, 19);
        assert_eq!(&handshake.bittorrent, b"BitTorrent protocol");
        Ok(Peer {
            addr: self.addr,
            id: Some(handshake.peer_id),
            session: Some(Framed::new(stream, MessageFramer)),
            handshake: PhantomData,
        })
    }
}

impl Peer<Handshook> {
    pub fn id(&self) -> &[u8; 20] {
        self.id.as_ref().expect("only None while Uninitialized")
    }

    pub async fn bitfield(mut self, piece: usize) -> anyhow::Result<Option<Peer<Bitfield>>> {
        let peer = self
            .session
            .as_mut()
            .expect("only None while Uninitialized");

        let bitfield = peer
            .next()
            .await
            .expect("peer always sends a bitfields")
            .context("peer message was invalid")?;
        assert_eq!(bitfield.tag, MessageTag::Bitfield);

        let pieces = bitfield
            .payload
            .iter()
            .flat_map(|byte| format!("{:b}", byte).chars().collect::<Vec<_>>())
            .enumerate()
            .filter_map(|(i, b)| (b == '1').then_some(i))
            .collect::<Vec<_>>();

        if !pieces.contains(&piece) {
            return Ok(None);
        }

        Ok(Some(Peer {
            addr: self.addr,
            id: self.id,
            session: self.session,
            handshake: PhantomData,
        }))
    }
}

impl Peer<Bitfield> {
    pub async fn interested(mut self) -> anyhow::Result<Peer<Ready>> {
        let peer = self
            .session
            .as_mut()
            .expect("only None while Uninitialized");

        peer.send(Message {
            tag: MessageTag::Interested,
            payload: Vec::new(),
        })
        .await
        .context("send interested message")?;

        let unchoke = peer
            .next()
            .await
            .expect("peer always sends an unchoke")
            .context("peer message was invalid")?;
        assert_eq!(unchoke.tag, MessageTag::Unchoke);
        assert!(unchoke.payload.is_empty());

        Ok(Peer {
            addr: self.addr,
            id: self.id,
            session: self.session,
            handshake: PhantomData,
        })
    }
}

impl Peer<Ready> {
    pub async fn request(&mut self, request: Request, blocks: &mut Vec<u8>) -> anyhow::Result<()> {
        let peer = self
            .session
            .as_mut()
            .expect("only None while Uninitialized");

        peer.send(Message {
            tag: MessageTag::Request,
            payload: Vec::from(request.as_bytes()),
        })
        .await
        .with_context(|| {
            format!(
                "send request with block {}",
                request.begin() as usize / BLOCK_MAX
            )
        })?;

        let piece = peer
            .next()
            .await
            .expect("peer always sends a piece")
            .context("peer message was invalid")?;
        assert_eq!(piece.tag, MessageTag::Piece);
        assert!(!piece.payload.is_empty());

        let piece = Piece::ref_from_bytes(&piece.payload[..])
            .expect("always get all Piece response fields from peer");
        assert_eq!(piece.index(), request.index());
        assert_eq!(piece.begin(), request.begin());

        blocks.extend(piece.block());

        Ok(())
    }
}

impl TryFrom<String> for Peer<Uninitialized> {
    type Error = <std::net::SocketAddrV4 as std::str::FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self::new(value.parse()?))
    }
}
