use crate::transformer::Transformer;

#[derive(Default)]
pub struct RsaTransformer {}

impl Transformer for RsaTransformer {
    type Error = String;

    fn update(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>, Self::Error> {
        Ok(bytes)
    }
}
