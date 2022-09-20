use std::fmt::Debug;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Reads target in chunks, transforms it using [Transformer::update], and writes to target.
/// Returns total bytes written or error.
pub async fn transform<R: AsyncRead + Send + Unpin + 'static, W: AsyncWrite + Unpin>(
    source: R,
    transformer: impl Transformer,
    message_len: usize,
    target: &mut W,
) -> usize {
    let (sender, receiver) = channel::<io::Result<Vec<u8>>>(16);
    spawn_reading(source, sender, message_len);
    transform_and_write(transformer, target, receiver).await
}

async fn transform_and_write<W: AsyncWrite + Unpin>(
    mut transformer: impl Transformer,
    target: &mut W,
    mut receiver: Receiver<io::Result<Vec<u8>>>,
) -> usize {
    let mut written = 0;
    while let Some(message) = receiver.recv().await {
        let bytes = message.unwrap();
        let res = transformer.update(bytes).expect("transformer error");
        if !res.is_empty() {
            written += res.len();
            target.write_all(&res).await.unwrap();
        }
    }
    written
}

fn spawn_reading<R: AsyncRead + Send + Unpin + 'static>(
    mut source: R,
    sender: Sender<io::Result<Vec<u8>>>,
    message_len: usize,
) {
    tokio::spawn(async move {
        loop {
            let mut buffer = vec![0u8; message_len]; // TODO use bufreader
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
pub trait Transformer {
    type Error: Debug + Send + Sync + 'static;

    /// Called repeatedly with new bytes from the source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - chunk of the source file
    ///
    /// Returns resulting bytes to be written to a target file or error.
    fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error>;
}
