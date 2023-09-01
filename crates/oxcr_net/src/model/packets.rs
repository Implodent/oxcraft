use ::bytes::{BufMut, Bytes, BytesMut};
use aott::input::SliceInput;
use aott::prelude::*;

use crate::ser::*;

use super::{LEB128Number, VarInt};

pub mod handshake;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketClientbound {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum PacketServerbound {
    Handshake(handshake::Handshake),
}

impl Deserialize for PacketServerbound {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, Self> {
        choice((handshake::Handshake::deserialize.map(Self::Handshake),)).parse(input)
    }
}

pub trait Packet: Deserialize + Serialize {
    const ID: super::VarInt;
    const STATE: super::State;
}

#[derive(Debug, Clone)]
pub struct SerializedPacket {
    pub length: super::VarInt,
    pub id: super::VarInt,
    pub data: Bytes,
}

impl Deserialize for SerializedPacket {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, Self> {
        tuple((VarInt::deserialize, VarInt::deserialize, slice_till_end))
            .map(|(length, id, data)| Self { length, id, data })
    }
}
impl Serialize for SerializedPacket {
    fn serialize(&self) -> Bytes {
        let mut b = BytesMut::with_capacity(VarInt::max_length() * 2 + self.data.len());
        b.put(self.length.write());
        b.put(self.id.write());
        b.put(&self.data);
        b.freeze()
    }
}
