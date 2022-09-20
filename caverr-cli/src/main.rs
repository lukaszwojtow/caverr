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

use caverr_lib::worker::handler::RsaHandler;
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

lazy_static! {
    static ref RUNNING: AtomicUsize = AtomicUsize::new(0);
}

#[tokio::main]
async fn main() {
    console_subscriber::init();
    start(RsaHandler::encryptor()).await;
}

async fn check_spawn(entry: PathBuf, transformer: &RsaHandler) {
    let mut sleep = 1;
    loop {
        let running = RUNNING.load(Ordering::SeqCst);
        if running > 1024 {
            println!("sleeping for {}s", sleep);
            tokio::time::sleep(Duration::from_secs(sleep)).await;
            sleep *= 2;
        } else {
            break;
        }
    }
    let transformer = transformer.clone();
    spawn_transforming_task(entry, transformer);
}

fn spawn_transforming_task(entry: PathBuf, transformer: RsaHandler) {
    RUNNING.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        transformer.transform(entry).await;
        RUNNING.fetch_sub(1, Ordering::SeqCst);
    });
}

async fn start(transformer: RsaHandler) {
    for i in 0..1_000_000 {
        check_spawn(PathBuf::from("/tmp/fff"), &transformer).await;
        if i % 100 == 0 {
            println!("i: {}", i);
        }
    }
}
