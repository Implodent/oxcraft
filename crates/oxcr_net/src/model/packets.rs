pub mod handshake;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketClientbound {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketServerbound {
    Handshake(handshake::Handshake),
}
