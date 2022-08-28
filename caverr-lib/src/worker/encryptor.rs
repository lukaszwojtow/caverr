use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::transformer::Transformer;
use crate::xor::XorCipher;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

pub struct EncryptorHandle {
    sender: mpsc::Sender<EncMessage>,
}

impl EncryptorHandle {
    pub fn new(key: Vec<u8>, target_root: &Path) -> Self {
        let (sender, receiver) = mpsc::channel(1024);
        let target_dir = target_root
            .canonicalize()
            .expect("Target directory doesn't exist");
        // TODO spawn more actors to handle encrypting more than one file at a time.
        let actor = EncryptorWorker::new(target_dir, key, receiver);
        tokio::spawn(start_loop(actor));
        Self { sender }
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
    key: Vec<u8>,
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
    fn new(target_dir: PathBuf, key: Vec<u8>, receiver: mpsc::Receiver<EncMessage>) -> Self {
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
        let xor = XorCipher::new(self.key.clone()).await?;
        let target_path = build_relative_path(path, &self.target_dir)?;
        file_transform(path, xor, &target_path).await
    }
}
