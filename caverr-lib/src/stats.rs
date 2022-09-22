use crossbeam::channel::{Receiver, Sender};
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

#[derive(Clone)]
pub struct StatHandler {
    sender: Sender<StatMessage>,
}

impl Default for StatHandler {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        thread::spawn(move || {
            let worker = StatWorker::new(receiver);
            start_loop(worker);
        });
        Self { sender }
    }
}

impl StatHandler {
    pub fn increment_count(&self) {
        self.sender
            .send(StatMessage::IncrementCount)
            .expect("Unable to send IncrementCount");
    }
    pub fn decrement_count(&self) {
        self.sender
            .send(StatMessage::DecrementCount)
            .expect("Unable to send DecrementCount");
    }

    pub fn update(&self, bytes: usize, path: PathBuf) {
        self.sender
            .send(StatMessage::Update(bytes, path))
            .expect("Unable to send stats update");
    }

    pub fn current(&self) -> CurrentStats {
        let (sender, receiver) = crossbeam::channel::unbounded();
        let request = StatMessage::Request(sender);
        self.sender
            .send(request)
            .expect("Unable to send stats request");
        receiver.recv().expect("Unable to read current stats")
    }
}

#[derive(Debug, Clone)]
pub struct CurrentStats {
    pub bytes: usize, // TODO visibility
    pub bytes_per_second: f32,
    pub files: usize,
    pub counter: usize,
    last: PathBuf,
}

fn start_loop(mut actor: StatWorker) {
    while let Ok(msg) = actor.receiver.recv() {
        actor.handle_message(msg);
    }
}

struct StatWorker {
    receiver: Receiver<StatMessage>,
    stats: CurrentStats,
    start: Instant,
}

#[derive(Debug)]
enum StatMessage {
    Update(usize, PathBuf),
    Request(Sender<CurrentStats>),
    IncrementCount,
    DecrementCount,
}

impl StatWorker {
    fn new(receiver: Receiver<StatMessage>) -> Self {
        StatWorker {
            start: Instant::now(),
            receiver,
            stats: CurrentStats {
                bytes_per_second: 0.0,
                bytes: 0,
                files: 0,
                counter: 0,
                last: Default::default(),
            },
        }
    }

    fn handle_message(&mut self, msg: StatMessage) {
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
            StatMessage::IncrementCount => self.stats.counter += 1,
            StatMessage::DecrementCount => self.stats.counter -= 1,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::stats::StatHandler;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn should_handle_stats() {
        let stats = StatHandler::default();
        sleep(Duration::from_secs(1));
        let current = stats.current();
        assert_eq!(current.bytes, 0);
        assert_eq!(current.files, 0);
        assert_eq!(current.last, PathBuf::from(""));

        stats.update(10, PathBuf::from("1"));
        stats.update(5, PathBuf::from("2"));

        sleep(Duration::from_secs(1));
        let current = stats.current();
        assert_eq!(current.bytes, 15);
        assert_eq!(current.files, 2);
        assert_eq!(current.last, PathBuf::from("2"));
    }
}
