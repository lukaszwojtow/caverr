use crate::worker::rsa::holder::RsaHolder;
use std::fs::File;
use std::io::Write;
use std::io::{BufReader, BufWriter, Read};

pub(super) fn file_transform(
    mut source: BufReader<File>,
    rsa: RsaHolder,
    message_len: usize,
    target: &mut BufWriter<File>,
) -> anyhow::Result<()> {
    loop {
        let mut buffer = vec![0u8; message_len];
        let len = source.read(&mut buffer[..])?;
        if len > 0 {
            buffer.truncate(len);
            let transformed = rsa.work(buffer)?;
            target.write_all(&transformed)?;
        } else {
            break;
        }
    }
    Ok(())
}
