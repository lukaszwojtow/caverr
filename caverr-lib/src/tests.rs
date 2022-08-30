use crate::worker::encryptor::EncryptorHandle;
use crate::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
use std::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn should_generate_keys_to_encrypt() {
    const ORIGINAL_FILE_NAME: &str = "original.txt";

    // Get keys.
    let (private_key, public_key) = generate_keys().await.expect("Unable to create keys");
    let test_dir = tempfile::TempDir::new().expect("Unable to create temp dir");

    // Create public key file.
    let public_key_path = test_dir.path().join("public.key");
    let mut public_key_file = File::create(&public_key_path)
        .await
        .expect("Unable to create file");
    write_public_key(&mut public_key_file, public_key)
        .await
        .expect("Unable to write public key");
    public_key_file.flush().await.expect("Unable to flush file");

    // Create private key file.
    let private_key_path = test_dir.path().join("private.key");
    let mut private_key_file = File::create(private_key_path)
        .await
        .expect("Unable to create file");
    write_private_key(&mut private_key_file, private_key)
        .await
        .expect("Unable to write private key");
    private_key_file
        .flush()
        .await
        .expect("Unable to flush file");

    // Create file to encrypt.
    let original_file_path = test_dir.path().join(ORIGINAL_FILE_NAME);
    let mut original_file = File::create(&original_file_path)
        .await
        .expect("Unable to create original content file");
    original_file
        .write_all(&content())
        .await
        .expect("Unable to write to original file");
    original_file.flush().await.expect("Unable to flash file");

    // Encrypt file.
    let target_dir = test_dir.path().join("target");
    fs::create_dir_all(&target_dir).expect("Unable to create target_dir");
    let encryptor =
        EncryptorHandle::new(&public_key_path, &target_dir).expect("Unable to create encryptor");
    let result = encryptor.encrypt(original_file_path.clone()).await;
    assert!(result.is_ok());
    let encrypted_path = result.unwrap().1;
    assert!(encrypted_path.is_file());

    // Create decryption target dir.
    let decrypted_target_dir = test_dir.path().join("decrypted");
    fs::create_dir_all(&decrypted_target_dir).expect("Unable to create decrypted_target_dir");

    // Decrypt file.
    // Compare content.
}

fn content() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1024 * 1024);
    for i in 0..1024 * 1024 {
        bytes.push((i % 256) as u8);
    }
    bytes
}
