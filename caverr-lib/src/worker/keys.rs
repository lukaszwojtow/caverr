use rand::thread_rng;
use rsa::pkcs1::LineEnding::CRLF;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};

pub async fn generate_keys() {
    let mut rng = thread_rng();
    let bits = 4096;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("Failed to generate a key");
    let public_key = RsaPublicKey::from(&private_key);
    let private_key_string = private_key
        .to_pkcs8_pem(CRLF)
        .expect("Unable to show private key");
    println!("{}", private_key_string.as_str());
    let public_key_string = public_key
        .to_public_key_pem(CRLF)
        .expect("Unable to show public key");
    println!("{}", public_key_string);
}
