use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::worker::rsa::transformer::{RsaKey, RsaTransformer};
use anyhow::Context;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

pub struct EncryptorHandle {
    sender: mpsc::Sender<EncMessage>,
}

impl EncryptorHandle {
    pub fn new(key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let public_key = RsaPublicKey::read_public_key_pem_file(key_file)
            .with_context(|| format!("Unable to read public key from file {:?}", key_file))?;
        let (sender, receiver) = mpsc::channel(1024);
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        // TODO spawn more actors to handle encrypting more than one file at a time.
        let actor = EncryptorWorker::new(target_dir, RsaKey::PublicKey(public_key), receiver);
        tokio::spawn(start_loop(actor));
        Ok(Self { sender })
    }

    pub async fn encrypt(&self, path: PathBuf) -> anyhow::Result<usize> {
        let (ret, rcv) = oneshot::channel();
        self.sender.send(EncMessage::Encrypt { path, ret }).await?;
        rcv.await?
    }
}

async fn start_loop(mut actor: EncryptorWorker) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

struct EncryptorWorker {
    receiver: mpsc::Receiver<EncMessage>,
    key: RsaKey,
    target_dir: PathBuf,
}

#[derive(Debug)]
enum EncMessage {
    // TODO change to struct
    Encrypt {
        path: PathBuf,
        ret: oneshot::Sender<anyhow::Result<usize>>,
    },
}

impl EncryptorWorker {
    fn new(target_dir: PathBuf, key: RsaKey, receiver: mpsc::Receiver<EncMessage>) -> Self {
        EncryptorWorker {
            key,
            receiver,
            target_dir,
        }
    }
    async fn handle_message(&mut self, msg: EncMessage) {
        match msg {
            EncMessage::Encrypt { path, ret } => {
                let result = self.encrypt(&path).await;
                ret.send(result).expect("Unable to send result from worker");
            }
        }
    }

    async fn encrypt(&self, path: &Path) -> anyhow::Result<usize> {
        let rsa = RsaTransformer::new(self.key.clone());
        let target_path = build_relative_path(path, &self.target_dir)?;
        file_transform(path, rsa, &target_path).await
    }
}
