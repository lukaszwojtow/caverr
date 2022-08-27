#![forbid(unsafe_code)]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

use crate::args::{Args, Command};
use caverr_lib::worker::keys::generate_keys;
use caverr_lib::worker::EncryptorHandle;
use clap::Parser;
use std::fs::read_dir;
use std::path::PathBuf;

const TARGET_ROOT: &str = "/tmp"; // TODO make program arg

mod args;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Backup => todo!(),
        Command::Decrypt => todo!(),
        Command::Encrypt => on_files(args.source.unwrap()).await,
        Command::Verify => todo!(),
        Command::GenKeys => generate_keys().await,
    }
}

async fn send_or_queue(entry: PathBuf, queue: &mut Vec<PathBuf>, encryptor: &EncryptorHandle) {
    // TODO return number of bytes process to show later to the user.
    if entry.is_file() {
        if let Err(e) = encryptor.encrypt(entry).await {
            eprintln!("Unable to process file: {:?}", e);
        }
    } else if entry.is_dir() {
        queue.push(entry);
    }
}

async fn on_files(entry: PathBuf) {
    let mut queue = Vec::with_capacity(1024);
    let encryptor = EncryptorHandle::new(vec![42], &PathBuf::from(TARGET_ROOT)); // TODO add real key
    send_or_queue(entry, &mut queue, &encryptor).await;
    while !queue.is_empty() {
        let path = queue.swap_remove(queue.len() - 1);
        match read_dir(&path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => send_or_queue(f.path(), &mut queue, &encryptor).await,
                        Err(ref e) => {
                            eprintln!("Unable to read path: {:?} {:?} {}", path, file, e)
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Unable to scan directory: {:?}: {}", path, e)
            }
        }
    }
    println!("All files processed. Exiting.");
}
