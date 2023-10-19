use crate::{
    error::Error,
    model::{State, VarInt},
    ser::*,
};
use aott::{bytes as b, prelude::parser};

use super::{Packet, PacketContext};

#[derive(Debug, Clone)]
pub struct Handshake {
    pub protocol_version: crate::model::VarInt,
    pub addr: FixedStr<255>,
    pub port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

impl Deserialize for Handshake {
    type Context = PacketContext;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let protocol_version = deser_cx(input)?;
        let addr = deser_cx(input)?;
        let port = b::number::big::u16(input)?;
        let VarInt(next_state) = deser_cx(input)?;

        Ok(Self {
            protocol_version,
            addr,
            port,
            next_state: match next_state {
                1 => HandshakeNextState::Status,
                2 => HandshakeNextState::Login,
                s => return Err(Error::InvalidStateId(s)),
            },
        })
    }
}

impl Serialize for Handshake {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        self.protocol_version.serialize_to(buf)?;
        self.addr.serialize_to(buf)?;
        self.port.serialize_to(buf)?;
        VarInt(self.next_state as i32).serialize_to(buf)?;
        Ok(())
    }
}

impl Packet for Handshake {
    const ID: crate::model::VarInt = crate::model::VarInt(0x00);
    const STATE: State = State::Handshaking;
}
