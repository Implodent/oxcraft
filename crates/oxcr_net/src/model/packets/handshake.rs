use crate::{model::State, ser::*};
use aott::prelude::*;

use super::Packet;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Handshake {
    pub protocol_version: crate::model::VarInt,
}

impl Deserialize for Handshake {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, Self> {
        let (input, protocol_version) = deser(input)?;
        Ok((input, Self { protocol_version }))
    }
}

impl Packet for Handshake {
    const ID: crate::model::VarInt = crate::model::VarInt(0x00);
    const STATE: State = State::Handshaking;
}
