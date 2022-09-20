use crate::file::file_transform;
use crate::worker::rsa::transformer::RsaTransformer;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct RsaHandler {
    senders: Vec<Sender<Message>>,
}

impl RsaHandler {
    pub fn encryptor() -> Self {
        let senders = Self::create_senders();
        Self { senders }
    }

    pub async fn transform(&self, path: PathBuf) -> Transformed {
        let (snd, rcv) = oneshot::channel();
        let sender = self.pick_sender();
        sender.send(Message { path, channel: snd }).await.unwrap();
        rcv.await.unwrap()
    }

    fn pick_sender(&self) -> &Sender<Message> {
        let mut hpv = self.senders[0].capacity();
        let mut hpi = 0;
        for i in 1..self.senders.len() {
            if self.senders[i].capacity() > hpv {
                hpv = self.senders[i].capacity();
                hpi = i;
            }
        }
        &self.senders[hpi]
    }

    fn create_senders() -> Vec<Sender<Message>> {
        (0..10)
            .into_iter()
            .map(|_| {
                let (sender, receiver) = mpsc::channel(1024);
                Self::start_worker(receiver);
                sender
            })
            .collect()
    }

    fn start_worker(receiver: Receiver<Message>) {
        let worker = RsaWorker::new(receiver);
        tokio::spawn(start_loop(worker));
    }
}

#[derive(Debug)]
pub enum Transformed {
    Processed(usize, PathBuf),
}

async fn start_loop(mut actor: RsaWorker) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

struct RsaWorker {
    receiver: Receiver<Message>,
}

#[derive(Debug)]
struct Message {
    path: PathBuf,
    channel: oneshot::Sender<Transformed>,
}

impl RsaWorker {
    fn new(receiver: Receiver<Message>) -> Self {
        RsaWorker { receiver }
    }

    async fn handle_message(&mut self, msg: Message) {
        let result = self.work(&msg.path).await;
        msg.channel
            .send(result)
            .expect("Unable to send result from worker");
    }

    async fn work(&self, source: &Path) -> Transformed {
        let target_path = PathBuf::from("/dev/null");
        let rsa = RsaTransformer::default();
        let bytes = file_transform(source, rsa, &target_path, 256).await;
        Transformed::Processed(bytes, target_path)
    }
}
