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
    Ok(written)
}

fn spawn_reading<R: AsyncRead + Send + 'static>(source: R, sender: Sender<io::Result<Vec<u8>>>) {
    tokio::spawn(async move {
        let mut source = Box::pin(source);
        loop {
            let mut buffer = vec![0u8; 256]; // TODO extend this buffer to at least one page (4096) or use bufreader
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

    /// Called repeatedly with new bytes from the source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - chunk of the source file
    ///
    /// Returns resulting bytes to be written to a target file or error.
    async fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error>;
}
