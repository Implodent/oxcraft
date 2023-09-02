use crate::error::Error;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::input::SliceInput;
use aott::prelude::*;

use crate::ser::*;

use self::handshake::Handshake;

use super::{LEB128Number, VarInt};

pub mod handshake;

#[derive(Debug, Clone)]
pub enum PacketClientbound {}

#[derive(Debug, Clone)]
pub enum PacketServerbound {
    Handshake(handshake::Handshake),
}

impl Deserialize for PacketServerbound {
    type Context = PacketContext;

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, PacketContext>,
    ) -> Resul<'parse, 'a, Self, PacketContext> {
        match input.context().id {
            Handshake::ID => Handshake::deserialize.map(Self::Handshake).parse(input),
            VarInt(id) => Err((input, Error::InvalidPacketId(id))),
        }
    }
}

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
}

impl Deserialize for SerializedPacket {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Resul<'parse, 'a, Self> {
        tuple((VarInt::deserialize, VarInt::deserialize, slice_till_end))
            .map(|(length, id, data)| Self {
                length,
                id,
                data: data.into(),
            })
            .parse(input)
    }
}
impl Serialize for SerializedPacket {
    fn serialize_to(&self, buf: &mut BytesMut) {
        buf.reserve(self.length.length_of() + self.id.length_of() + self.data.len());
        self.length.serialize_to(buf);
        self.id.serialize_to(buf);
        buf.put(self.data);
    }
}
