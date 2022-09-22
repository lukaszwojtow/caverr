use crate::transformer::{transform, Transformer};
use anyhow::Context;
use rand::{thread_rng, RngCore};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn file_transform(
    source_path: &Path,
    transformer: impl Transformer,
    target_path: &Path,
    message_len: usize,
) -> anyhow::Result<usize> {
    let source = File::open(&source_path)
        .with_context(|| format!("Unable to read the source file: {:?}", source_path))?;
    let tmp_path = target_path.with_file_name(format!("{}.tmp", thread_rng().next_u64()));
    let mut tmp_target = File::create(&tmp_path)
        .with_context(|| format!("Unable to write to target file: {:?}", tmp_path))?;
    let bytes =
        transform(source, transformer, message_len, &mut tmp_target).with_context(|| {
            format!(
                "Unable to transform file from {:?} to {:?}",
                source_path, target_path
            )
        })?;
    tmp_target
        .flush()
        .with_context(|| format!("Unable to flush file: {:?}", tmp_path))?;
    fs::rename(tmp_path, target_path)
        .with_context(|| format!("Unable to rename file to:  {:?}", target_path))?;
    Ok(bytes)
}
