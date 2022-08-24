use crate::transformer::Transformer;
use async_trait::async_trait;
use thiserror::Error;

pub struct XorCipher {
    seed: u8,
}

#[derive(Debug, Error)]
pub enum XorError {
    #[error("invalid seed length {0}, must be 1")]
    InvalidSeedLength(usize),
}

#[async_trait]
impl Transformer for XorCipher {
    type Error = XorError;

    async fn new(bytes: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        if bytes.len() != 1 {
            Err(XorError::InvalidSeedLength(bytes.len()))
        } else {
            Ok(Self { seed: bytes[0] })
        }
    }

    async fn update(&mut self, mut bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error> {
        bytes.iter_mut().for_each(|b| *b ^= self.seed);
        Ok(bytes)
    }

    async fn finish(mut self) -> Result<Vec<u8>, Self::Error> {
        Ok(Default::default())
    }
}
