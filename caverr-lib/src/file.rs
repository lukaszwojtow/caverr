use crate::worker::rsa::holder::RsaHolder;
use anyhow::Context;
use rand::{thread_rng, RngCore};
use rayon::iter::ParallelBridge;
use rayon::prelude::ParallelIterator;
use std::fs::File;
use std::io::{BufReader, Write};
use std::io::{BufWriter, Read};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::{fs, io};

pub fn file_transform(
    source_path: &Path,
    rsa: RsaHolder,
    target_path: &Path,
    message_len: usize,
) -> anyhow::Result<u64> {
    let source = File::open(&source_path)
        .with_context(|| format!("Unable to read the source file: {:?}", source_path))?;
    let bytes = source.metadata()?.len();
    let source = ParallelFile::new(source, message_len);
    let tmp_path = target_path.with_file_name(format!("{}.tmp", thread_rng().next_u64()));
    let tmp_target = File::create(&tmp_path)
        .with_context(|| format!("Unable to write to target file: {:?}", tmp_path))?;
    let buffered_target = Arc::new(Mutex::new(BufWriter::new(tmp_target)));
    let pending_chunks = PendingChunks::new();
    let error: Arc<RwLock<Option<anyhow::Error>>> = Arc::new(RwLock::new(None));
    source.into_iter().par_bridge().for_each(|chunk| {
        let error_lock = error.read().unwrap();
        if error_lock.is_some() {
            return;
        } else {
            drop(error_lock);
        };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                let mut error_lock = error.write().unwrap();
                *error_lock = Some(e.into());
                return;
            }
        };
        let transformed = match rsa.work(chunk.data) {
            Ok(bytes) => bytes,
            Err(e) => {
                let mut error_lock = error.write().unwrap();
                *error_lock = Some(e.into());
                return;
            }
        };
        let mut chunks = pending_chunks.inner.lock().unwrap();
        if chunks.next == chunk.id {
            let mut t = buffered_target.lock().unwrap();
            if let Err(e) = t.write_all(&transformed) {
                let mut error_lock = error.write().unwrap();
                *error_lock = Some(e.into());
                return;
            }
            chunks.next += 1;
            let mut next = chunks.next;
            while let Some(found) = chunks.find(next) {
                if let Err(e) = t.write_all(&found.data) {
                    let mut error_lock = error.write().unwrap();
                    *error_lock = Some(e.into());
                    return;
                }
                chunks.next += 1;
                next = chunks.next;
            }
        } else {
            chunks.push(Chunk {
                data: transformed,
                id: chunk.id,
            });
        }
    });
    let mut tmp_target = buffered_target.lock().unwrap();
    tmp_target
        .flush()
        .with_context(|| format!("Unable to flush file: {:?}", tmp_path))?;
    drop(tmp_target);
    fs::rename(tmp_path, target_path)
        .with_context(|| format!("Unable to rename file to:  {:?}", target_path))?;
    let error = Arc::try_unwrap(error).unwrap().into_inner().unwrap();
    if let Some(error) = error {
        Err(error)
    } else {
        Ok(bytes)
    }
}

struct ParallelFile {
    inner: Arc<Mutex<InnerParallelFile>>,
}

struct InnerParallelFile {
    file: BufReader<File>,
    chunk_size: usize,
    next_id: usize,
    was_error: bool,
}

impl ParallelFile {
    fn new(file: File, chunk_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerParallelFile {
                file: BufReader::with_capacity(65536, file),
                chunk_size,
                next_id: 0,
                was_error: false,
            })),
        }
    }
}

struct Chunk {
    data: Vec<u8>,
    id: usize,
}

impl Iterator for ParallelFile {
    type Item = io::Result<Chunk>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut inner = self.inner.lock().unwrap();
        if inner.was_error {
            return None;
        }
        let size = inner.chunk_size;
        let mut buffer = vec![0u8; size];
        let result = inner.file.read(&mut buffer[..]);
        let id = inner.next_id;
        inner.next_id += 1;
        drop(inner);

        match result {
            Ok(len) => {
                if len == 0 {
                    None
                } else {
                    buffer.truncate(len);
                    Some(Ok(Chunk { data: buffer, id }))
                }
            }
            Err(e) => {
                let mut inner = self.inner.lock().unwrap();
                inner.was_error = true;
                Some(Err(e))
            }
        }
    }
}

struct PendingChunks {
    inner: Arc<Mutex<InnerPendingChunks>>,
}

struct InnerPendingChunks {
    chunks: Vec<Chunk>,
    next: usize,
}

impl InnerPendingChunks {
    fn find(&mut self, id: usize) -> Option<Chunk> {
        for i in 0..self.chunks.len() {
            if self.chunks[i].id == id {
                return Some(self.chunks.remove(i));
            }
        }
        None
    }

    fn push(&mut self, chunk: Chunk) {
        self.chunks.push(chunk);
    }
}

impl PendingChunks {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerPendingChunks {
                chunks: Vec::with_capacity(32),
                next: 0,
            })),
        }
    }
}
