use crate::{
    model::{chat::ChatComponent, player::*, State, VarInt},
    ser::{Array, Identifier, Json, Serialize},
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

#[derive(Debug)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: GameMode,
    pub prev_game_mode: PreviousGameMode,
    pub dimension_names: Array<Identifier>,
}
