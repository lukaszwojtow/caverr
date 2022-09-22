use crate::transformer::Transformer;
use crate::worker::rsa::{DECRYPTION_MESSAGE_SIZE, ENCRYPTION_MESSAGE_SIZE};
use rand::thread_rng;
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use sha2::Sha256;

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum RsaKey {
    PublicKey(RsaPublicKey),
    PrivateKey(RsaPrivateKey),
}

impl RsaKey {
    pub fn message_len(&self) -> usize {
        match self {
            RsaKey::PublicKey(_) => ENCRYPTION_MESSAGE_SIZE,
            RsaKey::PrivateKey(_) => DECRYPTION_MESSAGE_SIZE,
        }
    }
}

pub struct RsaTransformer {
    key: RsaKey,
}

impl RsaTransformer {
    pub fn new(key: RsaKey) -> Self {
        Self { key }
    }
}

impl Transformer for RsaTransformer {
    type Error = rsa::errors::Error;

    fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error> {
        let mut rng = thread_rng();
        match &self.key {
            RsaKey::PublicKey(key) => Ok(key.encrypt(&mut rng, padding(), &bytes)?),
            RsaKey::PrivateKey(key) => Ok(key.decrypt(padding(), bytes.as_ref())?),
        }
    }
}

fn padding() -> PaddingScheme {
    PaddingScheme::new_oaep_with_mgf_hash::<Sha256, Sha1>()
}
