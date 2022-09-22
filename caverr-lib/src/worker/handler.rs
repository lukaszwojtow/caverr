use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::worker::rsa::transformer::{RsaKey, RsaTransformer};
use anyhow::Context;
use crossbeam::channel::{Receiver, Sender};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{io, thread};

#[derive(Clone)]
pub struct RsaHandler {
    sender: Sender<Message>,
}

impl RsaHandler {
    pub fn encryptor(public_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        let key = Self::prepare_public_key(public_key_file)?;
        let (sender, receiver) = crossbeam::channel::bounded(1024);
        Self::start_worker(receiver, target_dir, key);
        Ok(Self { sender })
    }

    pub fn decryptor(private_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        let key = Self::prepare_private_key(private_key_file)?;
        let (sender, receiver) = crossbeam::channel::bounded(1024);
        Self::start_worker(receiver, target_dir, key);
        Ok(Self { sender })
    }

    pub fn transform(&self, path: Arc<Mutex<PathBuf>>) -> anyhow::Result<Transformed> {
        let (snd, rcv) = crossbeam::channel::bounded(1);
        self.sender.send(Message { path, channel: snd })?;
        rcv.recv()?
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

    fn start_worker(receiver: Receiver<Message>, target_dir: PathBuf, key: RsaKey) {
        let worker = RsaWorker::new(target_dir, key, receiver);
        thread::spawn(|| {
            start_loop(worker);
        });
    }
}

#[derive(Debug)]
pub enum Transformed {
    Skipped,
    Processed(usize, PathBuf),
}

fn start_loop(mut actor: RsaWorker) {
    while let Ok(msg) = actor.receiver.recv() {
        actor.handle_message(msg);
    }
}

struct RsaWorker {
    receiver: Receiver<Message>,
    key: RsaKey,
    target_dir: PathBuf,
}

#[derive(Debug)]
struct Message {
    path: Arc<Mutex<PathBuf>>,
    channel: Sender<anyhow::Result<Transformed>>,
}

impl RsaWorker {
    fn new(target_dir: PathBuf, key: RsaKey, receiver: Receiver<Message>) -> Self {
        RsaWorker {
            key,
            receiver,
            target_dir,
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let path = msg.path.lock().unwrap(); //TODO
        let result = self.work(&path);
        msg.channel
            .send(result)
            .expect("Unable to send result from worker");
    }

    fn work(&self, source: &Path) -> anyhow::Result<Transformed> {
        let target_path = build_relative_path(source, &self.target_dir)?;
        if is_newer(source, &target_path).unwrap_or(true) {
            let rsa = RsaTransformer::new(self.key.clone());
            let bytes = file_transform(source, rsa, &target_path, self.key.message_len())?;
            Ok(Transformed::Processed(bytes, target_path))
        } else {
            Ok(Transformed::Skipped)
        }
    }
}

fn is_newer(source: &Path, target: &Path) -> io::Result<bool> {
    if !target.exists() {
        Ok(true)
    } else {
        let source_time = source.metadata()?.modified()?;
        let target_time = target.metadata()?.modified()?;
        Ok(source_time > target_time)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::fs::write;

    #[tokio::test]
    async fn should_check_for_newer() {
        let tmp = TempDir::new().expect("Unable to create TempDir");
        let first_path = tmp.path().join("first");
        write(&first_path, vec![1; 1024])
            .await
            .expect("Unable to write");

        tokio::time::sleep(Duration::from_secs(2)).await;
        let second_path = tmp.path().join("second");
        write(&second_path, vec![1; 1024])
            .await
            .expect("Unable to write");
        let check = is_newer(&first_path, &second_path);
        assert!(check.is_ok());
        assert!(!check.unwrap());

        let check = is_newer(&second_path, &first_path);
        assert!(check.is_ok());
        assert!(check.unwrap());

        let does_not_exist = PathBuf::from("does").join("not").join("exist");
        let check = is_newer(&does_not_exist, &first_path);
        assert!(check.is_err());

        let check = is_newer(&first_path, &does_not_exist);
        assert!(check.is_ok());
        assert!(check.unwrap());
    }
}
