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

use crate::args::{validate_args, Args, Command};
use crate::exit_codes::ExitCodes;
use caverr_lib::worker::handler::RsaHandler;
use caverr_lib::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
use clap::Parser;
use lazy_static::lazy_static;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, RwLock};
use tokio::signal::unix::{signal, SignalKind};

mod args;
mod exit_codes;

lazy_static! {
    static ref STATS: Arc<RwLock<Stats>> = Arc::new(RwLock::new(Stats {
        bytes: 0,
        files: 0,
        last: PathBuf::from("<no files processed yet>")
    }));
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(e) = validate_args(&args) {
        eprintln!("{e}");
        exit(ExitCodes::InvalidArgs as i32);
    }
    match args.command {
        Command::Backup => todo!(),
        Command::Decrypt => todo!(),
        Command::Encrypt => encrypt(args).await,
        Command::Verify => todo!(),
        Command::GenKeys => get_new_keys().await,
    }
}

async fn show_stats_at_signal() {
    let mut stream = signal(SignalKind::hangup()).expect("Unable to register signal handler");
    loop {
        stream.recv().await;
        let stats = STATS.read().expect("Unable to lock stats").clone();
        println!("Current stats: {:?}", stats);
    }
}

async fn encrypt(args: Args) {
    tokio::spawn(show_stats_at_signal());
    match RsaHandler::encryptor(&args.key.unwrap(), &args.target.unwrap()) {
        Ok(encryptor) => walk_dirs(args.source.unwrap(), encryptor).await,
        Err(e) => {
            eprintln!("Unable to create encryptor: {:?}", e);
            exit(ExitCodes::EncryptorError as i32)
        }
    }
}

async fn get_new_keys() {
    match generate_keys().await {
        Ok((private_key, public_key)) => {
            if let Err(e) = write_public_key(&mut tokio::io::stdout(), public_key).await {
                eprintln!("Unable to write public key: {:?}", e);
                exit(ExitCodes::UnableToWriteKeys as i32);
            }
            if let Err(e) = write_private_key(&mut tokio::io::stdout(), private_key).await {
                eprintln!("Unable to write private key: {:?}", e);
                exit(ExitCodes::UnableToWriteKeys as i32);
            }
        }

        Err(e) => {
            eprintln!("Unable to generate keys: {:?}", e);
            exit(ExitCodes::KeyGenerationError as i32);
        }
    }
}

async fn transform_or_queue(entry: PathBuf, queue: &mut Vec<PathBuf>, encryptor: &RsaHandler) {
    if entry.is_file() {
        match encryptor.transform(entry).await {
            Ok((bytes, path)) => {
                let mut stats = STATS.write().unwrap();
                stats.bytes += bytes;
                stats.files += 1;
                stats.last = path;
            }
            Err(e) => eprintln!("Unable to process file: {:?}", e),
        }
    } else if entry.is_dir() {
        queue.push(entry);
    }
}

async fn walk_dirs(entry: PathBuf, encryptor: RsaHandler) {
    let mut queue = Vec::with_capacity(1024);
    transform_or_queue(entry, &mut queue, &encryptor).await;
    while !queue.is_empty() {
        let path = queue.swap_remove(queue.len() - 1);
        match read_dir(&path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => transform_or_queue(f.path(), &mut queue, &encryptor).await,
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

#[derive(Clone, Debug)]
struct Stats {
    pub(crate) bytes: usize,
    pub(crate) files: usize,
    pub(crate) last: PathBuf,
}
