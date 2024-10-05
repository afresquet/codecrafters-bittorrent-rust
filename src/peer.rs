use std::{marker::PhantomData, net::SocketAddrV4};

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::Framed;

use crate::{message::*, torrent::BLOCK_MAX};

pub struct NoId;
pub struct Id([u8; 20]);

pub struct NoSession;
pub struct Session(Framed<TcpStream, MessageFramer>);

pub struct NoPieces;
pub struct Pieces(Vec<usize>);

pub struct NotReady;
pub struct Ready;

pub struct Peer<I, S, P, T> {
    addr: SocketAddrV4,
    id: I,
    session: S,
    pieces: P,
    state: PhantomData<T>,
}

impl Peer<NoId, NoSession, NoPieces, NotReady> {
    pub fn new(addr: SocketAddrV4) -> Self {
        Self {
            addr,
            id: NoId,
            session: NoSession,
            pieces: NoPieces,
            state: PhantomData,
        }
    }

    pub async fn handshake(
        self,
        info_hash: [u8; 20],
    ) -> anyhow::Result<Peer<Id, Session, NoPieces, NotReady>> {
        let mut stream = tokio::net::TcpStream::connect(self.addr).await?;
        let mut handshake = Handshake::new(info_hash);
        let bytes = handshake.as_bytes_mut();
        stream.write_all(bytes).await?;
        stream.read_exact(bytes).await?;
        anyhow::ensure!(handshake.length == 19);
        anyhow::ensure!(&handshake.bittorrent == b"BitTorrent protocol");
        Ok(Peer {
            addr: self.addr,
            id: Id(handshake.peer_id),
            session: Session(Framed::new(stream, MessageFramer)),
            pieces: self.pieces,
            state: PhantomData,
        })
    }
}

impl Peer<Id, Session, NoPieces, NotReady> {
    pub async fn bitfield(mut self) -> anyhow::Result<Peer<Id, Session, Pieces, NotReady>> {
        let bitfield = self
            .session_mut()
            .next()
            .await
            .expect("peer always sends a bitfields")
            .context("peer message was invalid")?;
        anyhow::ensure!(bitfield.tag == MessageTag::Bitfield);

        let pieces = bitfield
            .payload
            .iter()
            .flat_map(|byte| format!("{:b}", byte).chars().collect::<Vec<_>>())
            .enumerate()
            .filter_map(|(i, b)| (b == '1').then_some(i))
            .collect::<Vec<_>>();

        Ok(Peer {
            addr: self.addr,
            id: self.id,
            session: self.session,
            pieces: Pieces(pieces),
            state: PhantomData,
        })
    }
}

impl Peer<Id, Session, Pieces, NotReady> {
    pub async fn interested(mut self) -> anyhow::Result<Peer<Id, Session, Pieces, Ready>> {
        self.session_mut()
            .send(Message {
                tag: MessageTag::Interested,
                payload: Vec::new(),
            })
            .await
            .context("send interested message")?;

        let unchoke = self
            .session_mut()
            .next()
            .await
            .expect("peer always sends an unchoke")
            .context("peer message was invalid")?;
        anyhow::ensure!(unchoke.tag == MessageTag::Unchoke);
        anyhow::ensure!(unchoke.payload.is_empty());

        Ok(Peer {
            addr: self.addr,
            id: self.id,
            session: self.session,
            pieces: self.pieces,
            state: PhantomData,
        })
    }
}

impl Peer<Id, Session, Pieces, Ready> {
    pub async fn request(&mut self, request: Request, blocks: &mut Vec<u8>) -> anyhow::Result<()> {
        self.session_mut()
            .send(Message {
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

        let piece = self
            .session_mut()
            .next()
            .await
            .expect("peer always sends a piece")
            .context("peer message was invalid")?;
        anyhow::ensure!(piece.tag == MessageTag::Piece);
        anyhow::ensure!(!piece.payload.is_empty());

        let piece = Piece::ref_from_bytes(&piece.payload[..])
            .expect("always get all Piece response fields from peer");
        anyhow::ensure!(piece.index() == request.index());
        anyhow::ensure!(piece.begin() == request.begin());

        blocks.extend(piece.block());

        Ok(())
    }
}

impl<I, S, P, T> Peer<I, S, P, T> {
    pub fn addr(&self) -> &SocketAddrV4 {
        &self.addr
    }
}

impl<S, P, T> Peer<Id, S, P, T> {
    pub fn id(&self) -> &[u8; 20] {
        &self.id.0
    }
}

impl<I, P, T> Peer<I, Session, P, T> {
    pub fn session_mut(&mut self) -> &mut Framed<TcpStream, MessageFramer> {
        &mut self.session.0
    }
}

impl<I, S, T> Peer<I, S, Pieces, T> {
    pub fn pieces(&self) -> &[usize] {
        self.pieces.0.as_slice()
    }
}

impl TryFrom<String> for Peer<NoId, NoSession, NoPieces, NotReady> {
    type Error = <std::net::SocketAddrV4 as std::str::FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self::new(value.parse()?))
    }
}
