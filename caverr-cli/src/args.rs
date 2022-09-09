use crate::args::Command::{Decrypt, Encrypt};
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

    /// Target directory, must exist
    #[clap(short, long, value_parser)]
    pub(super) target: Option<PathBuf>,
}

pub(crate) fn validate_args(args: &Args) -> Result<(), String> {
    match args.command {
        GenKeys => {
            if !matches!(args.key, None) {
                Err("Error: `key` argument given when generating keys".to_string())
            } else if !matches!(args.source, None) {
                Err("Error: `source` argument given when generating keys".to_string())
            } else if !matches!(args.target, None) {
                Err("Error: `target` argument given when generating keys".to_string())
            } else {
                Ok(())
            }
        }
        Decrypt => Ok(()),
        Encrypt => Ok(()),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Command {
    GenKeys,
    Decrypt,
    Encrypt,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enc" => Ok(Encrypt),
            "dec" => Ok(Decrypt),
            "keys" => Ok(GenKeys),
            other => Err(format!("Invalid command `{}`. Must be either: `keys` to generate keys, `enc` for encryption, `dec` for decryption", other))
        }
    }
}
