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
use caverr_lib::worker::rsa::handler::{RsaHandler, Transformed};
use caverr_lib::worker::rsa::keys::{generate_keys, write_private_key, write_public_key};
use clap::Parser;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::fs::read_dir;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::thread;

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
        get_decryptor(&args.key.unwrap(), &args.target.unwrap())
    } else {
        get_encryptor(&args.key.unwrap(), &args.target.unwrap())
    };

    walk_dir(args.source.unwrap(), producer, stat_handler.clone());
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

fn walk_dir(source: PathBuf, rsa: RsaHandler, stats: StatHandler) {
    let mut files = Vec::with_capacity(1024);
    scan(source, &mut files, &stats);
    files
        .into_par_iter()
        .for_each(|file| transform_file(&rsa, file, &stats));
}

fn transform_file(rsa: &RsaHandler, file: PathBuf, stats: &StatHandler) {
    let transform_result = rsa.transform(&file);
    stats.decrement_count();
    match transform_result {
        Ok(transformed) => {
            if let Transformed::Processed(bytes, _) = transformed {
                let count = stats.current().counter;
                println!("Remaining: {} Last {:?}", count, file);
                stats.update(bytes, file);
            }
        }
        Err(e) => {
            eprintln!("Unable to process file {:?}: {:?}", file, e)
        }
    }
}

fn scan(entry: PathBuf, files: &mut Vec<PathBuf>, stats: &StatHandler) {
    if entry.is_symlink() {
        return;
    }
    if entry.is_file() {
        files.push(entry);
        stats.increment_count();
    } else if entry.is_dir() {
        match read_dir(&entry) {
            Ok(dir) => {
                for item in dir {
                    match item {
                        Ok(f) => scan(f.path(), files, stats),
                        Err(ref e) => {
                            eprintln!("Unable to read path {:?}: {}", entry, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Unable to scan directory {:?}: {}", entry, e);
            }
        }
    }
}
