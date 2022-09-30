mod multi_thread;

use crate::worker::rsa::holder::RsaHolder;
use anyhow::Context;
use rand::{thread_rng, RngCore};
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::{BufReader, Write};
use std::path::Path;

pub fn file_transform(
    source_path: &Path,
    rsa: RsaHolder,
    target_path: &Path,
    message_len: usize,
) -> anyhow::Result<u64> {
    let source = File::open(&source_path)
        .with_context(|| format!("Unable to read the source file: {:?}", source_path))?;
    let bytes = source.metadata()?.len();
    let source = BufReader::with_capacity(65536, source);
    let tmp_path = target_path.with_file_name(format!("{}.tmp", thread_rng().next_u64()));
    let mut tmp_target = BufWriter::with_capacity(
        65536,
        File::create(&tmp_path)
            .with_context(|| format!("Unable to write to target file: {:?}", tmp_path))?,
    );
    multi_thread::file_transform(source, rsa, message_len, &mut tmp_target)?;
    tmp_target
        .flush()
        .with_context(|| format!("Unable to flush file: {:?}", tmp_path))?;
    drop(tmp_target);
    fs::rename(tmp_path, target_path)
        .with_context(|| format!("Unable to rename file to:  {:?}", target_path))?;
    Ok(bytes)
}
