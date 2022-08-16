use crate::args::Command::{Backup, Decrypt, Encrypt, Verify};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub(super) struct Args {
    /// Main command
    #[clap(short, long, value_parser)]
    pub(super) command: Command,

    /// Key / password / setup  file
    #[clap(short, long, value_parser)]
    pub(super) key: PathBuf,

    /// Source file / directory
    #[clap(short, long, value_parser)]
    pub(super) source: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) enum Command {
    Backup,
    Decrypt,
    Encrypt,
    Verify,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enc" => Ok(Encrypt),
            "dec" => Ok(Decrypt),
            "bck" => Ok(Backup),
            "vrf" => Ok(Verify),
            other => Err(format!("Invalid command `{}`. Must be either: `enc` for encryption, `dec` for decryption, `bck` for backup or `vrf` for verify", other))
        }
    }
}
