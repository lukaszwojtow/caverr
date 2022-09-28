use crate::file::file_transform;
use crate::path::build_relative_path;
use crate::worker::rsa::holder::{RsaHolder, RsaKey};
use anyhow::Context;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RsaHandler {
    key: RsaKey,
    target_dir: PathBuf,
}

impl RsaHandler {
    pub fn encryptor(public_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        let key = Self::prepare_public_key(public_key_file)?;
        Ok(Self { key, target_dir })
    }

    pub fn decryptor(private_key_file: &Path, target_root: &Path) -> anyhow::Result<Self> {
        let target_dir = target_root
            .canonicalize()
            .with_context(|| "Target directory doesn't exist")?;
        let key = Self::prepare_private_key(private_key_file)?;
        Ok(Self { key, target_dir })
    }

    // TODO fix as_path calls
    pub fn transform(&self, path: &Path) -> anyhow::Result<Transformed> {
        let target_path = build_relative_path(path, self.target_dir.as_path())?;
        if is_newer(path, target_path.as_path()).unwrap_or(true) {
            // TODO fix clone()
            let rsa = RsaHolder::new(self.key.clone());
            let bytes = file_transform(path, rsa, target_path.as_path(), self.key.message_len())?;
            Ok(Transformed::Processed(bytes, target_path))
        } else {
            Ok(Transformed::Skipped)
        }
    }

    fn prepare_public_key(public_key_file: &Path) -> anyhow::Result<RsaKey> {
        let public_key =
            RsaPublicKey::read_public_key_pem_file(public_key_file).with_context(|| {
                format!("Unable to read public key from file {:?}", public_key_file)
            })?;
        Ok(RsaKey::PublicKey(public_key))
    }

    fn prepare_private_key(private_key_file: &Path) -> anyhow::Result<RsaKey> {
        let private_key =
            RsaPrivateKey::read_pkcs8_pem_file(private_key_file).with_context(|| {
                format!(
                    "Unable to read private key from file {:?}",
                    private_key_file
                )
            })?;
        Ok(RsaKey::PrivateKey(private_key))
    }
}

#[derive(Debug)]
pub enum Transformed {
    Skipped,
    Processed(u64, PathBuf),
}

fn is_newer(source: &Path, target: &Path) -> io::Result<bool> {
    if !target.exists() {
        Ok(true)
    } else {
        let source_time = source.metadata()?.modified()?;
        let target_time = target.metadata()?.modified()?;
        Ok(source_time > target_time)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::fs::write;

    // TODO remove tokio
    #[tokio::test]
    async fn should_check_for_newer() {
        let tmp = TempDir::new().expect("Unable to create TempDir");
        let first_path = tmp.path().join("first");
        write(&first_path, vec![1; 1024])
            .await
            .expect("Unable to write");

        tokio::time::sleep(Duration::from_secs(2)).await;
        let second_path = tmp.path().join("second");
        write(&second_path, vec![1; 1024])
            .await
            .expect("Unable to write");
        let check = is_newer(&first_path, &second_path);
        assert!(check.is_ok());
        assert!(!check.unwrap());

        let check = is_newer(&second_path, &first_path);
        assert!(check.is_ok());
        assert!(check.unwrap());

        let does_not_exist = PathBuf::from("does").join("not").join("exist");
        let check = is_newer(&does_not_exist, &first_path);
        assert!(check.is_err());

        let check = is_newer(&first_path, &does_not_exist);
        assert!(check.is_ok());
        assert!(check.unwrap());
    }
}
