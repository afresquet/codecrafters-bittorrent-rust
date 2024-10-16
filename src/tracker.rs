use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::peer::*;

/// Note: info_hash field is not included
#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub compact: u8,
}

impl TrackerRequest {
    pub fn new(left: usize) -> Self {
        Self {
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left,
            compact: 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    pub interval: usize,
    pub peers: Peers,
}

#[derive(Debug, Clone)]
pub struct Peers(Vec<SocketAddrV4>);

impl Peers {
    pub fn iter(&self) -> impl Iterator<Item = Peer<NoId, NoSession, NoPieces, NotReady>> + '_ {
        self.0.iter().map(|addr| Peer::new(*addr))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Serialize for Peers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut single_slice = Vec::with_capacity(self.0.len() * 6);

        for peer in &self.0 {
            single_slice.extend(peer.ip().octets());
            single_slice.extend(peer.port().to_be_bytes());
        }

        serializer.serialize_bytes(&single_slice)
    }
}

impl<'de> Deserialize<'de> for Peers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(PeersVisitor)
    }
}

struct PeersVisitor;

impl<'de> Visitor<'de> for PeersVisitor {
    type Value = Peers;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of peers composed of 4 bytes for IP and 2 bytes for port")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() % 6 != 0 {
            return Err(E::custom(format!("length is {}", v.len())));
        }

        let peers = v
            .chunks_exact(6)
            .map(|slice| {
                SocketAddrV4::new(
                    Ipv4Addr::new(slice[0], slice[1], slice[2], slice[3]),
                    u16::from_be_bytes([slice[4], slice[5]]),
                )
            })
            .collect();

        Ok(Peers(peers))
    }
}
