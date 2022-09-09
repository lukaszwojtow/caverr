use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::worker::rsa::transformer::{RsaKey, RsaTransformer};
use anyhow::Context;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::io;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

pub struct RsaHandler {
    sender: mpsc::Sender<Message>,
}

impl RsaHandler {
    pub fn encryptor(public_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::channel(1024);
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        // TODO spawn more actors to allow handling more than one file at a time.
        let worker = RsaWorker::new(
            target_dir,
            Self::prepare_public_key(public_key_file)?,
            receiver,
        );
        tokio::spawn(start_loop(worker));
        Ok(Self { sender })
    }

    pub fn decryptor(private_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::channel(1024);
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        // TODO spawn more actors to allow handling more than one file at a time.
        let actor = RsaWorker::new(
            target_dir,
            Self::prepare_private_key(private_key_file)?,
            receiver,
        );
        tokio::spawn(start_loop(actor));
        Ok(Self { sender })
    }

    pub async fn transform(&self, path: PathBuf) -> anyhow::Result<Transformed> {
        let (snd, rcv) = oneshot::channel();
        self.sender.send(Message { path, channel: snd }).await?;
        rcv.await?
    }

    fn prepare_public_key(public_key_file: &Path) -> anyhow::Result<RsaKey> {
        let public_key =
            RsaPublicKey::read_public_key_pem_file(public_key_file).with_context(|| {
                format!("Unable to read public key from file {:?}", public_key_file)
            })?;
        Ok(RsaKey::PublicKey(public_key))
    }

    fn prepare_private_key(private_key_file: &Path) -> anyhow::Result<RsaKey> {
        let private_key =
            RsaPrivateKey::read_pkcs8_pem_file(private_key_file).with_context(|| {
                format!(
                    "Unable to read private key from file {:?}",
                    private_key_file
                )
            })?;
        Ok(RsaKey::PrivateKey(private_key))
    }
}

#[derive(Debug)]
pub enum Transformed {
    Skipped,
    Processed(usize, PathBuf),
}

async fn start_loop(mut actor: RsaWorker) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

struct RsaWorker {
    receiver: mpsc::Receiver<Message>,
    key: RsaKey,
    target_dir: PathBuf,
}

#[derive(Debug)]
struct Message {
    path: PathBuf,
    channel: oneshot::Sender<anyhow::Result<Transformed>>,
}

impl RsaWorker {
    fn new(target_dir: PathBuf, key: RsaKey, receiver: mpsc::Receiver<Message>) -> Self {
        RsaWorker {
            key,
            receiver,
            target_dir,
        }
    }

    async fn handle_message(&mut self, msg: Message) {
        let result = self.work(&msg.path).await;
        msg.channel
            .send(result)
            .expect("Unable to send result from worker");
    }

    async fn work(&self, source: &Path) -> anyhow::Result<Transformed> {
        let target_path = build_relative_path(source, &self.target_dir)?;
        if needs_work(source, &target_path).unwrap_or(true) {
            let rsa = RsaTransformer::new(self.key.clone());
            let bytes = file_transform(source, rsa, &target_path, self.key.message_len()).await?;
            Ok(Transformed::Processed(bytes, target_path))
        } else {
            Ok(Transformed::Skipped)
        }
    }
}

fn needs_work(source: &Path, target: &Path) -> io::Result<bool> {
    if !target.exists() {
        Ok(true)
    } else {
        let source_time = source.metadata()?.modified()?;
        let target_time = target.metadata()?.modified()?;
        Ok(source_time > target_time)
    }
}
