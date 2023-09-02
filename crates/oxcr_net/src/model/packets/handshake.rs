use crate::{model::State, ser::*};
use aott::prelude::*;
use fstr::FStr;

use super::{Packet, PacketContext};

#[derive(Debug, Clone)]
pub struct Handshake {
    pub protocol_version: crate::model::VarInt,
    pub addr: FStr<255>,
}

impl Deserialize for Handshake {
    type Context = PacketContext;

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, PacketContext>,
    ) -> Resul<'parse, 'a, Self, PacketContext> {
        let (input, protocol_version) = deser_cx(input)?;
        let (input, addr) = deser_cx(input)?;
        Ok((
            input,
            Self {
                protocol_version,
                addr,
            },
        ))
    }
}

impl Packet for Handshake {
    const ID: crate::model::VarInt = crate::model::VarInt(0x00);
    const STATE: State = State::Handshaking;
}
