use crate::model::varint::VarInt;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Handshake {
    pub protocol_version: VarInt,
}
