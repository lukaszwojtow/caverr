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
use clap::Parser;
use crossbeam_channel::unbounded;
use futures::executor::block_on;
use std::fs::read_dir;
use std::path::PathBuf;

mod args;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Backup => todo!(),
        Command::Decrypt => todo!(),
        Command::Encrypt => on_files(args.source, show),
        Command::Verify => todo!(),
    }
}

async fn show(e: PathBuf) {
    println!("{:?}", e);
}
//
// async fn call_encrypt(source: PathBuf) {
//     println!("enc :{:?}", source);
//     let parent = source.parent().unwrap();
//     let target = Path::new(".").join(parent);
//     if !target.exists() {
//         if let Err(e) = fs::create_dir_all(&target) {
//             if e.kind() != ErrorKind::AlreadyExists {
//                 eprintln!("Unable to create target dir {:?}: {:?}", target, e);
//             }
//         }
//     }
//     let name = source.as_path().file_name().unwrap();
//     let target = target.join(name).join(".enc");
//     let transformer = XorCipher { seed: 1 };
//     file_transform(&source, transformer, &target).await.unwrap();
// }

fn on_files<F>(entry: PathBuf, action: fn(PathBuf) -> F)
where
    F: std::future::Future<Output = ()>,
{
    let (sender, receiver) = unbounded();
    split(entry, sender);
    std::thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| {
                while let Ok(path) = receiver.recv() {
                    block_on(action(path));
                }
            });
        }
    });
}

fn split(entry: PathBuf, sender: crossbeam_channel::Sender<PathBuf>) {
    if entry.is_dir() {
        match read_dir(&entry) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(f) => {
                            let path = f.path();
                            if path.is_file() {
                                sender.send(path).unwrap()
                            } else if path.is_dir() {
                                let sender = sender.clone();
                                tokio::spawn(async move {
                                    split(path, sender);
                                });
                            }
                        }
                        Err(ref e) => {
                            eprintln!("Unable to read entry: {:?} {:?} {}", entry, file, e)
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Unable to scan directory: {:?}: {}", entry, e)
            }
        }
    } else if entry.is_file() {
        sender.send(entry).unwrap();
    }
}
