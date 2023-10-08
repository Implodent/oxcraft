use std::{str::Utf8Error, string::FromUtf8Error};

use crate::{
    model::packets::SerializedPacket,
    ser::{SerializationError, WithSource},
};
use miette::Diagnostic;

#[derive(thiserror::Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    #[diagnostic(code(protocol::error::io))]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Ser(#[from] SerializationError<u8>),
    #[error(transparent)]
    #[diagnostic(transparent)]
    SerStr(#[from] SerializationError<char>),
    #[error(transparent)]
    #[diagnostic(transparent)]
    SerSrc(#[from] WithSource<u8>),
    #[error(transparent)]
    #[diagnostic(transparent)]
    SerStrSrc(#[from] WithSource<char>),
    #[error("Invalid packet ID: {_0}")]
    #[diagnostic(code(protocol::error::invalid_packet_id))]
    InvalidPacketId(i32),
    #[error("Packet send error: {_0:?}")]
    #[diagnostic(code(flume::error::send))]
    Send(#[from] flume::SendError<(bool, SerializedPacket)>),
    #[error("Packet send error: {_0:?}")]
    #[diagnostic(code(flume::error::send))]
    SendSingle(#[from] flume::SendError<SerializedPacket>),
    #[error("Packet receive error: {_0:?}")]
    #[diagnostic(code(flume::error::recv))]
    Recv(#[from] flume::RecvError),
    #[error("Invalid UTF-8 encountered: {_0}")]
    #[diagnostic(code(core::str::invalid_utf8))]
    InvalidUtf8(#[from] Utf8Error),
    #[error("Invalid UTF-8 encountered: {_0}")]
    #[diagnostic(code(std::string::invalid_utf8))]
    InvalidUtf8Str(#[from] FromUtf8Error),
    #[error("VarInt too big")]
    #[diagnostic(code(protocol::error::varint_too_big))]
    VarIntTooBig,
    #[error("Invalid state ID: {_0}")]
    #[diagnostic(
        code(protocol::error::invalid_state_id),
        help("there are only 2 states in a handshake packet - Status (encoded: 1i32), and Login (encoded: 2i32)"),
        url("https://wiki.vg/Protocol#Handshake")
    )]
    InvalidStateId(i32),
    #[error(transparent)]
    #[diagnostic(code(protocol::error::json))]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Nbt(#[from] crate::nbt::NbtError),
    #[error("Invalid bit flags encountered")]
    #[diagnostic(code(protocol::error::invalid_bit_flags))]
    InvalidBitFlags,
    #[error("Connection ended")]
    #[diagnostic(code(protocol::error::connection_reset))]
    ConnectionEnded,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
