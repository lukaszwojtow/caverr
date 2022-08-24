use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelativePathError {
    #[error("IO error {0}")]
    IOError(io::Error),
    #[error("invalid source path {0}")]
    InvalidSourcePath(String),
}

pub fn build_relative_path(source: &Path, target_dir: &Path) -> Result<PathBuf, RelativePathError> {
    let file_name = source.file_name().ok_or_else(|| {
        RelativePathError::InvalidSourcePath(format!("missing file name in path: {:?}", source))
    })?;
    let root = source.canonicalize().map_err(RelativePathError::IOError)?;
    let mut components = root
        .parent()
        .ok_or_else(|| {
            RelativePathError::InvalidSourcePath(format!("no parent in path {:?}", root))
        })?
        .components();
    components.next().ok_or_else(|| {
        RelativePathError::InvalidSourcePath(format!(
            "unable to remove root component from path {:?}",
            components.as_path()
        ))
    })?;
    let target_dir = target_dir.join(components.as_path());
    fs::create_dir_all(&target_dir).map_err(RelativePathError::IOError)?;
    Ok(target_dir.join(file_name))
}
