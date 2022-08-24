use crate::transformer::{transform, Transformer};
use anyhow::Context;
use std::path::Path;
use tokio::fs::File;

pub async fn file_transform(
    source_path: &Path,
    transformer: impl Transformer,
    target_path: &Path,
) -> anyhow::Result<usize> {
    let source = File::open(&source_path)
        .await
        .with_context(|| format!("Unable to read the source file: {:?}", source_path))?;
    let mut target = File::create(&target_path)
        .await
        .with_context(|| format!("Unable to write to target file: {:?}", target_path))?;
    transform(source, transformer, &mut target).await // TODO attach some context to error
}
