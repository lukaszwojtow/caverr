use rand::thread_rng;
use rsa::pkcs8::LineEnding::CRLF;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::io;
use thiserror::Error;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub async fn generate_keys() -> rsa::errors::Result<(RsaPrivateKey, RsaPublicKey)> {
    let mut rng = thread_rng();
    let bits = 4096;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;
    let public_key = RsaPublicKey::from(&private_key);
    Ok((private_key, public_key))
}

#[derive(Debug, Error)]
pub enum ShowKeyError {
    #[error("rsa error: {0}")]
    RsaError(rsa::pkcs8::Error),

    #[error("io error: {0}")]
    IOError(io::Error),
}

pub async fn write_public_key<W: AsyncWrite + Unpin>(
    w: &mut W,
    public_key: RsaPublicKey,
) -> Result<(), ShowKeyError> {
    let public_key_string = public_key
        .to_public_key_pem(CRLF)
        .map_err(|e| ShowKeyError::RsaError(rsa::pkcs8::Error::PublicKey(e)))?;
    w.write_all(public_key_string.as_bytes())
        .await
        .map_err(ShowKeyError::IOError)?;
    Ok(())
}

pub async fn write_private_key<W: AsyncWrite + Unpin>(
    w: &mut W,
    private_key: RsaPrivateKey,
) -> Result<(), ShowKeyError> {
    let private_key_string = private_key
        .to_pkcs8_pem(CRLF)
        .map_err(ShowKeyError::RsaError)?;
    w.write_all(private_key_string.as_bytes())
        .await
        .map_err(ShowKeyError::IOError)?;
    Ok(())
}
