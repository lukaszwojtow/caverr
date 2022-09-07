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
use caverr_lib::stats::StatHandler;
use caverr_lib::worker::handler::RsaHandler;
use caverr_lib::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
use clap::Parser;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::exit;
use tokio::signal::unix::{signal, SignalKind};

mod args;
mod exit_codes;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(e) = validate_args(&args) {
        eprintln!("{e}");
        exit(ExitCodes::InvalidArgs as i32);
    }
    match args.command {
        Command::Backup => todo!(),
        Command::Decrypt => decrypt(args).await,
        Command::Encrypt => encrypt(args).await,
        Command::Verify => todo!(),
        Command::GenKeys => get_new_keys().await,
    }
}

async fn show_stats_at_signal(handler: StatHandler) {
    let mut signals = signal(SignalKind::hangup()).expect("Unable to register signal handler");
    loop {
        signals.recv().await;
        let stats = handler.current().await;
        println!("{:?}", stats);
    }
}

fn get_decryptor(key: &Path, target: &Path) -> RsaHandler {
    match RsaHandler::decryptor(key, target) {
        Ok(decryptor) => decryptor,
        Err(e) => {
            eprintln!("Unable to create decryptor: {:?}", e);
            exit(ExitCodes::EncryptorError as i32)
        }
    }
}

fn get_encryptor(key: &Path, target: &Path) -> RsaHandler {
    match RsaHandler::encryptor(key, target) {
        Ok(decryptor) => decryptor,
        Err(e) => {
            eprintln!("Unable to create encryptor: {:?}", e);
            exit(ExitCodes::EncryptorError as i32)
        }
    }
}

fn start_stat_handler() -> StatHandler {
    let stat_handler = StatHandler::default();
    tokio::spawn(show_stats_at_signal(stat_handler.clone()));
    stat_handler
}

async fn decrypt(args: Args) {
    let stat_handler = start_stat_handler();
    let decryptor = get_decryptor(&args.key.unwrap(), &args.target.unwrap());
    walk_dirs(args.source.unwrap(), decryptor, stat_handler).await;
}

async fn encrypt(args: Args) {
    let stat_handler = start_stat_handler();
    let encryptor = get_encryptor(&args.key.unwrap(), &args.target.unwrap());
    walk_dirs(args.source.unwrap(), encryptor, stat_handler).await;
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

async fn transform_or_queue(
    entry: PathBuf,
    queue: &mut Vec<PathBuf>,
    encryptor: &RsaHandler,
    stats: &StatHandler,
) {
    if entry.is_file() {
        match encryptor.transform(entry).await {
            Ok((bytes, path)) => {
                stats.update(bytes, path).await;
            }
            Err(e) => eprintln!("Unable to process file: {:?}", e),
        }
    } else if entry.is_dir() {
        queue.push(entry);
    }
}

async fn walk_dirs(entry: PathBuf, transformer: RsaHandler, stats: StatHandler) {
    let mut queue = Vec::with_capacity(1024);
    transform_or_queue(entry, &mut queue, &transformer, &stats).await;
    while let Some(path) = queue.pop() {
        match read_dir(&path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => {
                            transform_or_queue(f.path(), &mut queue, &transformer, &stats).await
                        }
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
