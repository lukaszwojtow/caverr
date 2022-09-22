use crate::transformer::{transform, Transformer};
use anyhow::Context;
use std::fs::File;
use std::path::Path;

pub fn file_transform(
    source_path: &Path,
    transformer: impl Transformer,
    target_path: &Path,
    message_len: usize,
) -> anyhow::Result<usize> {
    let source = File::open(&source_path)
        .with_context(|| format!("Unable to read the source file: {:?}", source_path))?;
    let mut target = File::create(&target_path)
        .with_context(|| format!("Unable to write to target file: {:?}", target_path))?;
    transform(source, transformer, message_len, &mut target).with_context(|| {
        format!(
            "Unable to transform file from {:?} to {:?}",
            source_path, target_path
        )
    })
}
