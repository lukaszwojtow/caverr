pub mod keys;
pub mod transformer;

pub const KEY_BITS: usize = 4096;
pub const ENCRYPTION_MESSAGE_SIZE: usize = 256;
pub const DECRYPTION_MESSAGE_SIZE: usize = 512;

#[cfg(test)]
mod test {
    use crate::worker::handler::{RsaHandler, Transformed};
    use crate::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    const ORIGIN_CONTENT_LEN: usize = 16 * 1024;

    #[tokio::test]
    async fn should_generate_keys_to_encrypt_and_decrypt() {
        const ORIGINAL_FILE_NAME: &str = "original.txt";

        let start = Instant::now();

        // Get keys.
        let (private_key, public_key) = generate_keys().expect("Unable to create keys");
        println!("Generated keys after {:?}", start.elapsed());
        let test_dir = tempfile::TempDir::new().expect("Unable to create temp dir");

        // Create public key file.
        let public_key_path = test_dir.path().join("public.key");
        let mut public_key_file = File::create(&public_key_path).expect("Unable to create file");
        write_public_key(&mut public_key_file, public_key).expect("Unable to write public key");
        public_key_file.flush().expect("Unable to flush file");

        // Create private key file.
        let private_key_path = test_dir.path().join("private.key");
        let mut private_key_file = File::create(&private_key_path).expect("Unable to create file");
        write_private_key(&mut private_key_file, private_key).expect("Unable to write private key");
        private_key_file.flush().expect("Unable to flush file");

        // Create file to encrypt.
        let original_file_path = test_dir.path().join(ORIGINAL_FILE_NAME);
        let mut original_file =
            File::create(&original_file_path).expect("Unable to create original content file");
        original_file
            .write_all(&content())
            .expect("Unable to write to original file");
        original_file.flush().expect("Unable to flash file");
        println!("Created files after {:?}", start.elapsed());

        // Encrypt file.
        let target_dir = test_dir.path().join("target");
        fs::create_dir_all(&target_dir).expect("Unable to create target_dir");
        let encryptor = RsaHandler::encryptor(&public_key_path, &target_dir)
            .expect("Unable to create encryptor");
        let arc_original_file_path = Arc::new(Mutex::new(original_file_path.clone()));
        println!("Created encryptor after {:?}", start.elapsed());
        let result = encryptor
            .transform(arc_original_file_path)
            .expect("unable to transform");
        let encrypted = if let Transformed::Processed(bytes, path) = result {
            (bytes, path)
        } else {
            panic!("Result is not 'processed'");
        };
        assert!(encrypted.1.is_file());
        println!("Encrypted after {:?}", start.elapsed());

        // Decrypt file.
        let decrypted_target_dir = test_dir.path().join("decrypted");
        fs::create_dir_all(&decrypted_target_dir).expect("Unable to create decrypted_target_dir");
        let decryptor = RsaHandler::decryptor(&private_key_path, &decrypted_target_dir)
            .expect("Unable to create decryptor");
        println!("Created decryptor after {:?}", start.elapsed());
        let result = decryptor
            .transform(Arc::new(Mutex::new(encrypted.1)))
            .expect("unable to transform");
        let decrypted = if let Transformed::Processed(bytes, path) = result {
            (bytes, path)
        } else {
            panic!("Result is not 'processed'");
        };
        let decrypted_path = decrypted.1;
        assert!(decrypted_path.is_file());
        println!("Decrypted after {:?}", start.elapsed());

        // Compare content.
        let origin_content = fs::read(&original_file_path).expect("Unable to read origin file");
        let decrypted_content = fs::read(&decrypted_path).expect("Unable to read decrypted file");
        assert_eq!(origin_content, decrypted_content);
        assert_eq!(origin_content.len(), ORIGIN_CONTENT_LEN);
        println!("Compared after {:?}", start.elapsed());
    }

    fn content() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(ORIGIN_CONTENT_LEN);
        for i in 0..ORIGIN_CONTENT_LEN {
            bytes.push((i % 256) as u8);
        }
        bytes
    }
}
