use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use hashes::Hashes;
use sha1::{Digest, Sha1};

use crate::tracker::{TrackerRequest, TrackerResponse};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

impl Torrent {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        serde_bencode::from_bytes(&data).context("failed deserializing")
    }

    pub fn hash(&self) -> anyhow::Result<[u8; 20]> {
        let info_bencoded = serde_bencode::to_bytes(&self.info).context("re-encoding")?;
        let mut hasher = Sha1::new();
        hasher.update(&info_bencoded);
        Ok(hasher.finalize().into())
    }

    pub fn piece_hashes(&self) -> impl Iterator<Item = String> + '_ {
        self.info.pieces.iter().map(hex::encode)
    }

    pub async fn peers(&self) -> anyhow::Result<TrackerResponse> {
        let Keys::SingleFile { length } = self.info.keys;
        let info_hash = self.hash()?;

        let tracker_request = TrackerRequest {
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: length,
            compact: 1,
        };

        let url_params = serde_urlencoded::to_string(&tracker_request)
            .context("url-encode tracker parameters")?;
        let url = format!(
            "{}?{}&info_hash={}",
            self.announce,
            url_params,
            urlencode(&info_hash)
        );
        let tracker_url = reqwest::Url::parse(&url).context("parse tracker announce URL")?;

        let tracker_response: TrackerResponse = {
            let response = reqwest::get(tracker_url).await.context("query tracker")?;
            let bytes = response.bytes().await.context("fetch tracker")?;
            serde_bencode::from_bytes(&bytes).context("parse tracker")?
        };

        Ok(tracker_response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub name: String,

    #[serde(rename = "piece length")]
    pub piece_length: usize,

    pub pieces: Hashes,

    #[serde(flatten)]
    pub keys: Keys,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Keys {
    SingleFile { length: usize },
}

mod hashes {
    use std::fmt;
    use std::ops::Deref;

    use serde::{
        de::{self, Visitor},
        ser::{Serialize, Serializer},
        Deserialize, Deserializer,
    };

    #[derive(Debug)]
    pub struct Hashes(Vec<[u8; 20]>);

    impl Deref for Hashes {
        type Target = Vec<[u8; 20]>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Serialize for Hashes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let single_slice = self.0.concat();
            serializer.serialize_bytes(&single_slice)
        }
    }

    impl<'de> Deserialize<'de> for Hashes {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_bytes(HashesVisitor)
        }
    }

    struct HashesVisitor;

    impl<'de> Visitor<'de> for HashesVisitor {
        type Value = Hashes;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a list of segments of 20 bytes")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() % 20 != 0 {
                return Err(E::custom(format!("length is {}", v.len())));
            }

            Ok(Hashes(
                v.chunks_exact(20)
                    .map(|s| s.try_into().expect("is length 20"))
                    .collect(),
            ))
        }
    }
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(t.len() * 3);

    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode([byte]));
    }

    encoded
}
