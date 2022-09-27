use crate::worker::rsa::transformer::RsaTransformer;
use anyhow::Context;
use rand::{thread_rng, RngCore};
use rayon::iter::ParallelBridge;
use rayon::prelude::ParallelIterator;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};
use std::io::{BufWriter, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub fn file_transform(
    source_path: &Path,
    transformer: RsaTransformer,
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
    source.into_iter().par_bridge().for_each(|chunk| {
        let transformed = transformer.update(chunk.data).unwrap();
        let mut chunks = pending_chunks.inner.lock().unwrap();
        if chunks.next == chunk.id {
            let mut t = buffered_target.lock().unwrap();
            t.write_all(&transformed).unwrap(); // TODO handle
            chunks.next += 1;
            let mut next = chunks.next;
            while let Some(found) = chunks.find(next) {
                t.write_all(&found.data).unwrap(); // TODO handle
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
    Ok(bytes)
}

struct ParallelFile {
    inner: Arc<Mutex<InnerParallelFile>>,
}

struct InnerParallelFile {
    file: BufReader<File>,
    chunk_size: usize,
    next_id: usize,
}

impl ParallelFile {
    fn new(file: File, chunk_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerParallelFile {
                file: BufReader::with_capacity(65536, file),
                chunk_size,
                next_id: 0,
            })),
        }
    }
}

struct Chunk {
    data: Vec<u8>,
    id: usize,
}

impl Iterator for ParallelFile {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let mut inner = self.inner.lock().unwrap();
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
                    Some(Chunk { data: buffer, id })
                }
            }
            Err(_) => None, // TODO return error
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
                chunks: vec![],
                next: 0,
            })),
        }
    }
}
