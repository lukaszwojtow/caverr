use async_trait::async_trait;
use std::error::Error;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Reads target in chunks, transforms it using [Transformer::update], and writes to target.
/// Returns total bytes written or error.
pub async fn transform<R: AsyncRead + Send + 'static, W: AsyncWrite + Unpin>(
    source: R,
    transformer: impl Transformer,
    target: &mut W,
) -> anyhow::Result<usize> {
    let (sender, receiver) = channel::<io::Result<Vec<u8>>>(16);
    spawn_reading(source, sender);
    transform_and_write(transformer, target, receiver).await
}

async fn transform_and_write<W: AsyncWrite + Unpin>(
    mut transformer: impl Transformer,
    target: &mut W,
    mut receiver: Receiver<io::Result<Vec<u8>>>,
) -> anyhow::Result<usize> {
    let mut written = 0;
    while let Some(message) = receiver.recv().await {
        let bytes = message?;
        let res = transformer.update(bytes).await?;
        if !res.is_empty() {
            written += res.len();
            target.write_all(&res).await?;
        }
    }
    let res = transformer.finish().await?;

    if !res.is_empty() {
        written += res.len();
        target.write_all(&res).await?
    }
    Ok(written)
}

fn spawn_reading<R: AsyncRead + Send + 'static>(source: R, sender: Sender<io::Result<Vec<u8>>>) {
    tokio::spawn(async move {
        let mut source = Box::pin(source);
        loop {
            let mut buffer = vec![0u8; 4096];
            match source.read(&mut buffer).await {
                Ok(len) if len == 0 => return,
                Ok(len) => {
                    buffer.truncate(len);
                    sender
                        .send(Ok(buffer))
                        .await
                        .expect("Unable to send buffer to reader channel");
                }
                Err(e) => sender
                    .send(Err(e))
                    .await
                    .expect("Unable to send error to reader channel"),
            }
        }
    });
}

/// Main encryption / decryption trait. Algorithms have to implement one for encryption and one for decryption.
#[async_trait]
pub trait Transformer {
    type Error: Error + Send + Sync + 'static;
    /// Creates new [Transformer]. Called once per source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - Content of initial state file. Can be key, passphrase, configuration, etc...
    ///
    /// Returns Self or error.
    async fn new(bytes: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Called repeatedly with new bytes from the source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - chunk of the source file
    ///
    /// Returns resulting bytes to be written to a target file or error.
    async fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error>;

    /// Called when finished reading the source file.
    /// Can return bytes to be added to the target file.
    async fn finish(self) -> Result<Vec<u8>, Self::Error>;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;
    use thiserror::Error;

    struct XorCipher {
        seed: u8,
    }

    #[derive(Debug, Error)]
    enum XorError {
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

    #[tokio::test]
    async fn should_transform() {
        const LEN: usize = 5000;
        let xor = XorCipher::new(vec![0b_0101_0101_u8]).await.unwrap();
        let buffer = Cursor::new(vec![0b_0000_1111_u8; LEN]);
        let mut target = vec![];
        let bytes = transform(buffer, xor, &mut target).await.unwrap();
        // assert length
        assert_eq!(target.len(), LEN);
        assert_eq!(bytes, LEN);
        for byte in target {
            // assert each byte
            assert_eq!(byte, 0b_0101_1010_u8)
        }
    }
}
