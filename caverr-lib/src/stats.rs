use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

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
    pub async fn inc_queue_size(&self) {
        self.sender
            .send(StatMessage::IncRunning)
            .await
            .expect("Unable to send inc_queue_size");
    }
    pub async fn dec_queue_size(&self) {
        self.sender
            .send(StatMessage::DecRunning)
            .await
            .expect("Unable to send dec_queue_size");
    }

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
    pub bytes: usize,
    pub bytes_per_second: f32,
    pub files: usize,
    pub running_count: usize,
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
    start: Instant,
}

#[derive(Debug)]
enum StatMessage {
    Update(usize, PathBuf),
    Request(oneshot::Sender<CurrentStats>),
    IncRunning,
    DecRunning,
}

impl StatWorker {
    fn new(receiver: mpsc::Receiver<StatMessage>) -> Self {
        StatWorker {
            start: Instant::now(),
            receiver,
            stats: CurrentStats {
                bytes_per_second: 0.0,
                bytes: 0,
                files: 0,
                running_count: 0,
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
                let seconds = self.start.elapsed().as_secs_f32();
                self.stats.bytes_per_second = (self.stats.bytes as f32) / seconds;
                channel
                    .send(self.stats.clone())
                    .expect("Unable to send message to channel");
            }
            StatMessage::IncRunning => self.stats.running_count += 1,
            StatMessage::DecRunning => self.stats.running_count -= 1,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::stats::StatHandler;
    use std::path::PathBuf;
    use std::time::Duration;

    #[tokio::test]
    async fn should_handle_stats() {
        let stats = StatHandler::default();
        tokio::time::sleep(Duration::from_secs(1)).await;
        let current = stats.current().await;
        assert_eq!(current.bytes, 0);
        assert_eq!(current.files, 0);
        assert_eq!(current.last, PathBuf::from(""));

        stats.update(10, PathBuf::from("1")).await;
        stats.update(5, PathBuf::from("2")).await;

        tokio::time::sleep(Duration::from_secs(1)).await;
        let current = stats.current().await;
        assert_eq!(current.bytes, 15);
        assert_eq!(current.files, 2);
        assert_eq!(current.last, PathBuf::from("2"));
    }
}
