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
        GenKeys => validate_get_keys(args),
        Decrypt | Encrypt => validate_transform(args),
    }
}

fn validate_transform(args: &Args) -> Result<(), String> {
    if args.key.is_none() {
        Err("Error: `key` argument not given".into())
    } else if args.source.is_none() {
        Err("Error: `source` argument not given".into())
    } else if args.target.is_none() {
        Err("Error: `target` argument not given".into())
    } else {
        Ok(())
    }
}

fn validate_get_keys(args: &Args) -> Result<(), String> {
    if !matches!(args.key, None) {
        Err("Error: `key` argument given when generating keys".into())
    } else if !matches!(args.source, None) {
        Err("Error: `source` argument given when generating keys".into())
    } else if !matches!(args.target, None) {
        Err("Error: `target` argument given when generating keys".into())
    } else {
        Ok(())
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
