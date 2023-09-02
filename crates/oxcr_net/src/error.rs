use std::str::Utf8Error;

use crate::model::packets::SerializedPacket;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("duplicate player IP")]
    DupePlayer,
    #[error("IO error: {_0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {_0:?}")]
    Ser(aott::extra::Simple<u8>),
    #[error("Invalid packet ID: {_0}")]
    InvalidPacketId(i32),
    #[error("Packet send error: {_0:?}")]
    Send(#[from] tokio::sync::mpsc::error::SendError<SerializedPacket>),
    #[error("Packet send error: {_0:?}")]
    SendSend(#[from] tokio::sync::broadcast::error::SendError<SerializedPacket>),
    #[error("Packet receive error: {_0:?}")]
    Recv(#[from] tokio::sync::broadcast::error::RecvError),
    #[error("Invalid UTF-8 encountered: {_0}")]
    InvalidUtf8(#[from] Utf8Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
