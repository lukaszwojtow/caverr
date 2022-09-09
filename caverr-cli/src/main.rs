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

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use crate::args::{validate_args, Args, Command};
use crate::exit_codes::ExitCodes;
use caverr_lib::stats::StatHandler;
use caverr_lib::worker::handler::{RsaHandler, Transformed};
use caverr_lib::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
use clap::Parser;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::exit;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinHandle;

mod args;
mod exit_codes;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(e) = validate_args(&args) {
        eprintln!("{e}");
        exit(ExitCodes::InvalidArgs as i32);
    }
    if args.command == Command::GenKeys {
        get_new_keys().await;
        exit(0);
    }
    let start = std::time::Instant::now();
    let transformer = if args.command == Command::Decrypt {
        get_decryptor(&args.key.unwrap(), &args.target.unwrap())
    } else {
        get_encryptor(&args.key.unwrap(), &args.target.unwrap())
    };
    let stat_handler = start_stat_handler();
    walk_dirs(args.source.unwrap(), transformer, stat_handler.clone()).await;
    let stats = stat_handler.current().await;
    println!(
        "Processed {} files ({} bytes) in {} seconds.",
        stats.files,
        stats.bytes,
        start.elapsed().as_secs()
    );
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
    transformer: &RsaHandler,
    stats: &StatHandler,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    if entry.is_file() {
        let transformer = transformer.clone();
        let stats = stats.clone();
        let task = spawn_transforming_task(entry, transformer, stats);
        tasks.push(task);
    } else if entry.is_dir() {
        queue.push(entry);
    }
}

fn spawn_transforming_task(
    entry: PathBuf,
    transformer: RsaHandler,
    stats: StatHandler,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        match transformer.transform(entry).await {
            Ok(transformed) => {
                if let Transformed::Processed(bytes, path) = transformed {
                    stats.update(bytes, path).await
                }
            }
            Err(e) => eprintln!("Unable to process file: {:?}", e),
        }
    })
}

async fn walk_dirs(entry: PathBuf, transformer: RsaHandler, stats: StatHandler) {
    let mut queue = Vec::with_capacity(1024);
    let mut tasks = Vec::with_capacity(1024);
    transform_or_queue(entry, &mut queue, &transformer, &stats, &mut tasks).await;
    while let Some(path) = queue.pop() {
        match read_dir(&path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => {
                            transform_or_queue(
                                f.path(),
                                &mut queue,
                                &transformer,
                                &stats,
                                &mut tasks,
                            )
                            .await
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
    futures::future::join_all(tasks).await;
    println!("All files processed. Exiting.");
}
