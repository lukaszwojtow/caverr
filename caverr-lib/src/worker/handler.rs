use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::worker::rsa::transformer::{RsaKey, RsaTransformer};
use anyhow::Context;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
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
        let public_key =
            RsaPublicKey::read_public_key_pem_file(public_key_file).with_context(|| {
                format!("Unable to read public key from file {:?}", public_key_file)
            })?;
        let actor = RsaWorker::new(target_dir, RsaKey::PublicKey(public_key), receiver);
        tokio::spawn(start_loop(actor));
        Ok(Self { sender })
    }

    pub async fn transform(&self, path: PathBuf) -> anyhow::Result<(usize, PathBuf)> {
        let (ret, rcv) = oneshot::channel();
        self.sender.send(Message { path, ret }).await?;
        rcv.await?
    }
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
    ret: oneshot::Sender<anyhow::Result<(usize, PathBuf)>>,
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
        msg.ret
            .send(result)
            .expect("Unable to send result from worker");
    }

    async fn work(&self, path: &Path) -> anyhow::Result<(usize, PathBuf)> {
        let rsa = RsaTransformer::new(self.key.clone());
        let target_path = build_relative_path(path, &self.target_dir)?;
        let bytes = file_transform(path, rsa, &target_path).await?;
        Ok((bytes, target_path))
    }
}
