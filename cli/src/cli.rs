//! Spanned-clap, lol

use std::path::PathBuf;

use aott::prelude::*;
use oxcr_protocol::{aott, bytes::Bytes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliSerOp {
    Encode,
    Decode,
}

#[derive(Debug, Clone)]
pub enum CliSerOperand {
    File(PathBuf),
    Data(Bytes),
}

#[derive(Debug, Clone)]
pub struct CliSer {
    pub operation: CliSerOp,
    pub operand: CliSerOperand,
}

#[derive(Debug, Clone)]
pub enum Cli {
    Serialization(CliSer),
}

#[parser(extras = Extra)]
pub fn yay(input: &str) -> Cli {}
