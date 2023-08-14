use bytes::Bytes;

pub mod handshake;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketClientbound {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketServerbound {
    Handshake(handshake::Handshake),
}

pub trait Packet {
    const ID: super::VarInt;
    const STATE: super::State;
}

#[derive(Debug, Clone)]
pub struct SerializedPacket {
    pub length: super::VarInt,
    pub id: super::VarInt,
    pub data: Bytes,
}
