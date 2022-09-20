use crate::transformer::{transform, Transformer};
use std::path::Path;
use tokio::fs::File;

pub async fn file_transform(
    source_path: &Path,
    transformer: impl Transformer,
    target_path: &Path,
    message_len: usize,
) -> usize {
    let source = File::open(&source_path).await.unwrap();
    let mut target = File::create(&target_path).await.unwrap();
    transform(source, transformer, message_len, &mut target).await
}
