use crate::args::Command::{Backup, Decrypt, Encrypt, Verify};
use crate::Command::GenKeys;
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
    pub(super) key: Option<PathBuf>,

    /// Source file / directory
    #[clap(short, long, value_parser)]
    pub(super) source: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub(super) enum Command {
    GenKeys,
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
            "keys" => Ok(GenKeys),
            other => Err(format!("Invalid command `{}`. Must be either: `keys` to generate keys, `enc` for encryption, `dec` for decryption, `bck` for backup or `vrf` for verify", other))
        }
    }
}
