use crossbeam::channel::{Receiver, Sender};
use std::error::Error;
use std::io::{Read, Write};
use std::{io, thread};

// TODO change to bytes read instead
/// Reads target in chunks, transforms it using [Transformer::update], and writes to target.
/// Returns total bytes written or error.
// TODO change to use buffered IO
pub fn transform<R: Read + Send + 'static, W: Write + Unpin>(
    source: R,
    transformer: impl Transformer,
    message_len: usize,
    target: &mut W,
) -> anyhow::Result<usize> {
    let (sender, receiver) = crossbeam::channel::bounded::<io::Result<Vec<u8>>>(16);
    spawn_reading(source, sender, message_len);
    transform_and_write(transformer, target, receiver)
}

fn transform_and_write<W: Write + Unpin>(
    mut transformer: impl Transformer,
    target: &mut W,
    receiver: Receiver<io::Result<Vec<u8>>>,
) -> anyhow::Result<usize> {
    let mut written = 0;
    while let Ok(message) = receiver.recv() {
        let bytes = message?;
        let res = transformer.update(bytes)?;
        if !res.is_empty() {
            written += res.len();
            target.write_all(&res)?;
        }
    }
    Ok(written)
}

fn spawn_reading<R: Read + Send + 'static>(
    mut source: R,
    sender: Sender<io::Result<Vec<u8>>>,
    message_len: usize,
) {
    thread::spawn(move || {
        loop {
            let mut buffer = vec![0u8; message_len]; // TODO use bufreader
            match source.read(&mut buffer) {
                Ok(len) if len == 0 => return,
                Ok(len) => {
                    buffer.truncate(len);
                    sender
                        .send(Ok(buffer))
                        .expect("Unable to send buffer to reader channel");
                }
                Err(e) => sender
                    .send(Err(e))
                    .expect("Unable to send error to reader channel"),
            }
        }
    });
}

/// Main encryption / decryption trait. Algorithms have to implement one for encryption and one for decryption.
pub trait Transformer {
    type Error: Error + Send + Sync + 'static;

    /// Called repeatedly with new bytes from the source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - chunk of the source file
    ///
    /// Returns resulting bytes to be written to a target file or error.
    fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error>;
}
