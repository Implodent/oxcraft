use crate::error::Error;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::prelude::*;

use crate::ser::*;

use self::handshake::Handshake;

use super::{State, VarInt};

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

#[derive(Debug, Clone)]
pub enum PacketClientbound {}

#[derive(Debug, Clone)]
pub enum PacketServerbound {
    Handshake(handshake::Handshake),
}

impl Deserialize for PacketServerbound {
    type Context = PacketContext;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        match input.context().id {
            id if id == Handshake::ID => Handshake::deserialize
                .map(Self::Handshake)
                .parse_with(input),
            VarInt(id) => Err(Error::InvalidPacketId(id)),
        }
    }
}

#[derive(Debug)]
pub struct PacketContext {
    pub id: VarInt<i32>,
    pub state: super::State,
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

impl SerializedPacket {
    pub fn new<P: Packet + Serialize>(packet: P) -> Self {
        let data = packet.serialize();
        let id = P::ID;
        let length = VarInt((id.length_of() + data.len()) as i32);
        Self { length, id, data }
    }

    pub fn try_deserialize<P: Packet + Deserialize<Context = PacketContext>>(
        &self,
        state: State,
    ) -> Result<P, crate::error::Error> {
        let context = PacketContext { id: self.id, state };

        P::deserialize.parse_with_context(self.data.as_ref(), context)
    }
}

impl Deserialize for SerializedPacket {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        (
            VarInt::deserialize,
            VarInt::deserialize,
            slice_till_end.map(Bytes::copy_from_slice),
        )
            .map(|(length, id, data)| Self { length, id, data })
            .parse_with(input)
    }
}
impl Serialize for SerializedPacket {
    fn serialize_to(&self, buf: &mut BytesMut) {
        buf.reserve(self.length.length_of() + self.id.length_of() + self.data.len());
        self.length.serialize_to(buf);
        self.id.serialize_to(buf);
        buf.put_slice(&self.data);
    }
}

#[derive(Debug, Clone)]
pub struct PluginMessage {
    pub channel: Identifier,
    pub data: Bytes,
}

impl_ser!(|PacketContext| PluginMessage => [channel, data]);

impl Packet for PluginMessage {
    const ID: super::VarInt = VarInt(0x17);
    const STATE: super::State = State::Play;
}
