use crate::chapter::ChapterError;

use regex::Regex;
use sha2::{ Digest, Sha256 };

lazy_static! {
    static ref HASH_RE:Regex = Regex::new(r"([0-9a-fA-F]{64})").unwrap();
}

#[derive(Debug, Clone)]
pub struct Image {
    url: String,
    hash: Vec<u8>,
}
impl Image {
    pub fn new(chapter_hash:&str, filename:&str) -> Result<Self, ChapterError> {
        Ok(Self {
            url: format!("/data/{}/{}", chapter_hash, filename),
            hash: hex::decode(HASH_RE
                    .captures(filename)
                    .ok_or(ChapterError::HashNotFound)?
                    .get(0)
                    .ok_or(ChapterError::HashNotFound)?
                    .as_str()).unwrap(), // Guaranteed to be valid hex thanks to regex
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn verify(&self, body:&[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(body);
        let result = hasher.finalize();

        result[..] == self.hash
    }
}
