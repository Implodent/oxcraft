use std::{str::Utf8Error, string::FromUtf8Error};

use crate::{model::packets::SerializedPacket, ser::SerializationError};
use miette::Diagnostic;

#[derive(thiserror::Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(code(protocol::error::io))]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(code(protocol::error::parse))]
    Ser(#[from] SerializationError<u8>),
    #[error(transparent)]
    #[diagnostic(code(protocol::error::parse_str))]
    SerStr(#[from] SerializationError<char>),
    #[error("Invalid packet ID: {_0}")]
    #[diagnostic(code(protocol::error::invalid_packet_id))]
    InvalidPacketId(i32),
    #[error("Packet send error: {_0:?}")]
    #[diagnostic(code(protocol::error::flume_send))]
    Send(#[from] flume::SendError<SerializedPacket>),
    #[error("Packet receive error: {_0:?}")]
    Recv(#[from] flume::RecvError),
    #[error("Invalid UTF-8 encountered: {_0}")]
    #[diagnostic(code(protocol::error::invalid_utf8))]
    InvalidUtf8(#[from] Utf8Error),
    #[error("Invalid UTF-8 encountered: {_0}")]
    #[diagnostic(code(protocol::error::invalid_utf8_owned))]
    InvalidUtf8Str(#[from] FromUtf8Error),
    #[error("VarInt too big")]
    #[diagnostic(code(protocol::error::varint_too_big))]
    VarIntTooBig,
    #[error("Invalid state ID: {_0}")]
    InvalidStateId(i32),
    #[error(transparent)]
    #[diagnostic(code(protocol::error::json))]
    Json(#[from] serde_json::Error),
    #[error("NBT fucked up")] // xd
    NbtFuckup,
    #[error("Invalid bit flags encountered")]
    InvalidBitFlags,
    #[error("connection ended")]
    ConnectionEnded,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
