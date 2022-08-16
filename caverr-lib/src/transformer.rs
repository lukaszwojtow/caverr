use async_trait::async_trait;

/// Main encryption / decryption trait. Algorithms have to implement one for encryption and one for decryption.
#[async_trait]
pub trait Transformer {
    type Error;
    /// Creates new [Transformer]. Called once per source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - Content of initial state file. Can be key, passphrase, configuration, etc...
    ///
    /// Returns Self or error.
    async fn new(bytes: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Called repeatedly with new bytes from the source file.
    ///
    /// Accepts:
    ///
    /// `bytes` - chunk of target file
    ///
    /// Returns resulting bytes to be written to a target file or error.
    async fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error>;

    /// Called when finished reading the source file.
    /// Can return bytes to be added to the target file.
    async fn finish(&mut self) -> Result<Vec<u8>, Self::Error>;
}

#[cfg(test)]
mod test {
    use super::*;
    struct XorCipher(u8);

    #[async_trait]
    impl Transformer for XorCipher {
        type Error = String;

        async fn new(bytes: Vec<u8>) -> Result<Self, Self::Error>
        where
            Self: Sized,
        {
            if bytes.len() != 1 {
                Err(format!(
                    "Invalid length for XOR seed: {}, must be 1",
                    bytes.len()
                ))
            } else {
                Ok(Self(bytes[0]))
            }
        }

        async fn update(&mut self, mut bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error> {
            bytes.iter_mut().for_each(|b| *b ^= self.0);
            Ok(bytes)
        }

        async fn finish(&mut self) -> Result<Vec<u8>, Self::Error> {
            Ok(Default::default())
        }
    }

    #[tokio::test]
    async fn test_xor() {
        let mut xor = XorCipher::new(vec![0b_0101_0101_u8]).await.unwrap();
        let buffer = vec![0b_0000_1111_u8];
        let response = xor.update(buffer).await.unwrap();
        assert_eq!(response.len(), 1);
        assert_eq!(response[0], 0b_0101_1010_u8);
    }
}
