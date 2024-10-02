use anyhow::Context;
use serde::{Deserialize, Serialize};

use hashes::Hashes;
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

impl Torrent {
    pub fn hash(&self) -> anyhow::Result<String> {
        let info_bencoded = serde_bencode::to_bytes(&self.info).context("re-encoding")?;
        let mut hasher = Sha1::new();
        hasher.update(&info_bencoded);
        let info_hashed = hasher.finalize();
        Ok(hex::encode(info_hashed))
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

    struct HashesVisitor;

    impl<'de> Visitor<'de> for HashesVisitor {
        type Value = Hashes;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer between -2^31 and 2^31")
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

    impl<'de> Deserialize<'de> for Hashes {
        fn deserialize<D>(deserializer: D) -> Result<Hashes, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_bytes(HashesVisitor)
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
}
