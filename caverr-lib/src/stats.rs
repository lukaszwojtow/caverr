use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct StatHandler {
    sender: mpsc::Sender<StatMessage>,
}

impl Default for StatHandler {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel(1024);
        let worker = StatWorker::new(receiver);
        tokio::spawn(start_loop(worker));
        Self { sender }
    }
}

impl StatHandler {
    pub async fn update(&self, bytes: usize, path: PathBuf) {
        self.sender
            .send(StatMessage::Update(bytes, path))
            .await
            .expect("Unable to send stats update");
    }

    pub async fn current(&self) -> CurrentStats {
        let (sender, receiver) = oneshot::channel();
        let request = StatMessage::Request(sender);
        self.sender
            .send(request)
            .await
            .expect("Unable to send stats request");
        receiver.await.expect("Unable to read current stats")
    }
}

#[derive(Debug, Clone)]
pub struct CurrentStats {
    bytes: usize,
    files: usize,
    last: PathBuf,
}

async fn start_loop(mut actor: StatWorker) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

struct StatWorker {
    receiver: mpsc::Receiver<StatMessage>,
    stats: CurrentStats,
}

#[derive(Debug)]
enum StatMessage {
    Update(usize, PathBuf),
    Request(oneshot::Sender<CurrentStats>),
}

impl StatWorker {
    fn new(receiver: mpsc::Receiver<StatMessage>) -> Self {
        StatWorker {
            receiver,
            stats: CurrentStats {
                bytes: 0,
                files: 0,
                last: Default::default(),
            },
        }
    }

    async fn handle_message(&mut self, msg: StatMessage) {
        match msg {
            StatMessage::Update(bytes, file) => {
                self.stats.files += 1;
                self.stats.bytes += bytes;
                self.stats.last = file;
            }
            StatMessage::Request(channel) => {
                channel
                    .send(self.stats.clone())
                    .expect("Unable to send message to channel");
            }
        }
    }
}
