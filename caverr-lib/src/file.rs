use crate::transformer::{transform, TransformError, Transformer};
use std::fmt::Debug;
use std::path::PathBuf;
use tokio::fs::File;

pub async fn file_transform<E: Debug>(
    source: &PathBuf,
    transformer: impl Transformer<Error = E>,
    target: &PathBuf,
) -> Result<usize, TransformError<E>> {
    let source = File::open(source)
        .await
        .map_err(|e| TransformError::IOError(e))?;
    let mut target = File::create(target)
        .await
        .map_err(|e| TransformError::IOError(e))?;
    transform(source, transformer, &mut target).await
}
