use crate::model::State;

use super::Packet;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Handshake {
    pub protocol_version: crate::model::VarInt,
}

impl Packet for Handshake {
    const ID: crate::model::VarInt = crate::model::VarInt(0x00);
    const STATE: State = State::Handshaking;
}
