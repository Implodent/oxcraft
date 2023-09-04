use crate::{
    model::{chat::ChatComponent, State, VarInt},
    ser::{Json, Serialize},
};

use super::Packet;

#[derive(Debug)]
pub struct DisconnectPlay {
    pub reason: Json<ChatComponent>,
}

impl Packet for DisconnectPlay {
    const ID: crate::model::VarInt = VarInt(0x1A);
    const STATE: crate::model::State = State::Play;
}

impl Serialize for DisconnectPlay {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.reason.serialize_to(buf);
    }
}
