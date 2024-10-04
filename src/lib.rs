use std::ops::Deref;

use sha1::{Digest, Sha1};

pub mod bencode;
pub mod message;
pub mod peer;
pub mod torrent;
pub mod tracker;

pub struct Hash([u8; 20]);

impl Hash {
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }
}

impl Deref for Hash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
