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
use crossbeam::channel::Receiver;
use rayon::ThreadPoolBuilder;
use std::fs::read_dir;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

mod args;
mod exit_codes;

fn main() {
    let args = Args::parse();
    if let Err(e) = validate_args(&args) {
        eprintln!("{e}");
        exit(ExitCodes::InvalidArgs as i32);
    }
    if args.command == Command::GenKeys {
        get_new_keys();
        exit(0);
    }
    let start = std::time::Instant::now();
    let stat_handler = start_stat_handler();
    let producer = if args.command == Command::Decrypt {
        get_decryptor
    } else {
        get_encryptor
    };

    walk_dir(
        args.source.unwrap(),
        args.key.unwrap(),
        args.target.unwrap(),
        producer,
        stat_handler.clone(),
    );
    let stats = stat_handler.current();
    println!(
        "Processed {} files ({} bytes) in {} seconds.",
        stats.files,
        stats.bytes,
        start.elapsed().as_secs()
    );
}

fn show_stats_at_signal(handler: StatHandler) {
    use signal_hook::consts::SIGHUP;
    use signal_hook::iterator::Signals;

    let signals = Signals::new(&[SIGHUP]);
    thread::spawn(move || {
        for _ in signals.expect("Unable to register signals").forever() {
            let stats = handler.current();
            println!("{:?}", stats);
        }
    });
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
    show_stats_at_signal(stat_handler.clone());
    stat_handler
}

fn get_new_keys() {
    match generate_keys() {
        Ok((private_key, public_key)) => {
            if let Err(e) = write_public_key(&mut stdout(), public_key) {
                eprintln!("Unable to write public key: {:?}", e);
                exit(ExitCodes::UnableToWriteKeys as i32);
            }
            if let Err(e) = write_private_key(&mut stdout(), private_key) {
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

fn transform_or_queue(
    entry: PathBuf,
    path_sender: &mut crossbeam::channel::Sender<PathBuf>,
    queue: &mut Vec<PathBuf>,
    stats: &StatHandler,
) {
    if entry.is_symlink() {
        return;
    }
    if entry.is_file() {
        stats.increment_count();
        path_sender.send(entry).expect("unable to send path");
    } else if entry.is_dir() {
        queue.push(entry);
    }
}

fn walk_dir(
    source: PathBuf,
    key: PathBuf,
    target: PathBuf,
    producer: fn(&Path, &Path) -> RsaHandler,
    stats: StatHandler,
) {
    let (mut path_sender, path_receiver) = crossbeam::channel::bounded(1024);
    let path_reading_thread =
        spawn_path_reading(key, target, producer, path_receiver, stats.clone());
    let mut queue = Vec::with_capacity(1024);
    transform_or_queue(source, &mut path_sender, &mut queue, &stats);
    while let Some(path) = queue.pop() {
        match read_dir(&path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => transform_or_queue(f.path(), &mut path_sender, &mut queue, &stats),
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
    drop(path_sender);
    path_reading_thread
        .join()
        .expect("unable to join main thread");
}

fn spawn_path_reading(
    key: PathBuf,
    target: PathBuf,
    producer: fn(&Path, &Path) -> RsaHandler,
    path_receiver: Receiver<PathBuf>,
    thread_stats: StatHandler,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let pool = ThreadPoolBuilder::new()
            .num_threads(12)
            .build()
            .expect("cannot create a thread pool");

        pool.scope(|scope| {
            while let Ok(path) = path_receiver.recv() {
                scope.spawn(|_| {
                    let transformer = producer(&key, &target);
                    let path = Arc::new(Mutex::new(path)); // TODO remove arc
                    match transformer.transform(path.clone()) {
                        Ok(transformed) => {
                            thread_stats.decrement_count();
                            if let Transformed::Processed(bytes, path) = transformed {
                                let count = thread_stats.current().counter;
                                println!("Remaining: {} Last {:?}", count, path);
                                thread_stats.update(bytes, path);
                            }
                        }
                        Err(e) => {
                            eprintln!("Unable to process file {:?}: {:?}", path.lock().unwrap(), e)
                        }
                    }
                })
            }
        });
    })
}
