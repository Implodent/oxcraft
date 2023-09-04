use crate::{
    model::{State, VarInt},
    ser::*,
};
use aott::{
    bytes as b,
};

use super::{Packet, PacketContext};

#[derive(Debug, Clone)]
pub struct Handshake {
    pub protocol_version: crate::model::VarInt,
    pub addr: FixedStr<255>,
    pub port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Debug, Clone, Copy)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

impl Deserialize for Handshake {
    type Context = PacketContext;

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, PacketContext>,
    ) -> Resul<'parse, 'a, Self, PacketContext> {
        let (input, protocol_version) = deser_cx(input)?;
        let (input, addr) = deser_cx(input)?;
        let (input, port) = b::number::big::u16(input)?;
        let _offs = input.input.len();
        let (input, VarInt(next_state)) = deser_cx(input)?;

        Ok((
            input,
            Self {
                protocol_version,
                addr,
                port,
                next_state: match next_state {
                    1 => HandshakeNextState::Status,
                    2 => HandshakeNextState::Login,
                    s => panic!("Invalid NextState: {s}"),
                },
            },
        ))
    }
}

impl Packet for Handshake {
    const ID: crate::model::VarInt = crate::model::VarInt(0x00);
    const STATE: State = State::Handshaking;
}
