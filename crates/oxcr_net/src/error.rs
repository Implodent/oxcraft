use std::{str::Utf8Error, string::FromUtf8Error};

use crate::model::packets::SerializedPacket;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("duplicate player IP")]
    DupePlayer,
    #[error("IO error: {_0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {_0:?}")]
    Ser(aott::extra::Simple<u8>),
    #[error("Serialization error: {_0:?}")]
    SerStr(aott::extra::Simple<char>),
    #[error("Invalid packet ID: {_0}")]
    InvalidPacketId(i32),
    #[error("Packet send error: {_0:?}")]
    Send(#[from] flume::SendError<SerializedPacket>),
    #[error("Packet receive error: {_0:?}")]
    Recv(#[from] flume::RecvError),
    #[error("Invalid UTF-8 encountered: {_0}")]
    InvalidUtf8(#[from] Utf8Error),
    #[error("Invalid UTF-8 encountered: {_0}")]
    InvalidUtf8Str(#[from] FromUtf8Error),
    #[error("VarInt too big")]
    VarIntTooBig,
    #[error("Invalid state ID: {_0}")]
    InvalidStateId(i32),
    #[error("JSON error: {_0}")]
    Json(#[from] serde_json::Error),
    #[error("NBT fucked up")] // xd
    NbtFuckup,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
