use crate::worker::keys::{generate_keys, write_private_key, write_public_key};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn should_generate_keys_to_encrypt_and_decrypt() {
    let (private_key, public_key) = generate_keys().await.expect("Unable to create keys");
    let test_dir = tempfile::TempDir::new().expect("Unable to create temp dir");

    let mut public_key_file = File::create(test_dir.path().join("public.key"))
        .await
        .expect("Unable to create file");
    write_public_key(&mut public_key_file, public_key)
        .await
        .expect("Unable to write public key");
    public_key_file.flush().await.expect("Unable to flush file");

    let mut private_key_file = File::create(test_dir.path().join("private.key"))
        .await
        .expect("Unable to create file");
    write_private_key(&mut private_key_file, private_key)
        .await
        .expect("Unable to write private key");
    private_key_file
        .flush()
        .await
        .expect("Unable to flush file");
}
