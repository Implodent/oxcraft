use crate::error::Error;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::prelude::*;
use miette::NamedSource;

use crate::ser::*;

use super::{State, VarInt};

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

#[derive(Debug, Clone)]
pub enum PacketClientbound {}

macro define_packet_serverbound($($name:ident => $packet:path),*) {
    #[derive(Debug, Clone)]
    pub enum PacketServerbound {
        $($name($packet),)*
    }

    impl Deserialize for PacketServerbound {
        type Context = PacketContext;

        #[parser(extras = "Extra<Self::Context>")]
        fn deserialize(input: &[u8]) -> Self {
            match input.context().id {
                $(id if id == <$packet>::ID => <$packet>::deserialize
                    .map(Self::$name)
                  .parse_with(input),)*
                VarInt(id) => Err(Error::InvalidPacketId(id)),
            }
        }
    }
}

define_packet_serverbound![Handshake => handshake::Handshake];

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
    pub length: usize,
    pub id: super::VarInt,
    pub data: Bytes,
}

impl SerializedPacket {
    pub fn new<P: Packet + Serialize>(packet: P) -> Result<Self, Error> {
        Self::new_ref(&packet)
    }

    pub fn new_ref<P: Packet + Serialize + ?Sized>(packet: &P) -> Result<Self, Error> {
        try {
            let data = packet.serialize()?;
            let id = P::ID;
            let length = id.length_of() + data.len();
            Self { length, id, data }
        }
    }

    pub fn try_deserialize<P: Packet + Deserialize<Context = PacketContext>>(
        &self,
        state: State,
    ) -> Result<P, Error> {
        let context = PacketContext { id: self.id, state };

        P::deserialize
            .parse_with_context(self.data.as_ref(), context)
            .map_err(|e| match e {
                Error::Ser(error) => Error::SerSrc(WithSource {
                    source: NamedSource::new(format!("packet_{}", self.id), self.data.to_vec()),
                    error,
                }),
                e => e,
            })
    }
}

impl Deserialize for SerializedPacket {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        try {
            let length_varint = VarInt::<i32>::deserialize(input)?;
            assert!(length_varint.0 >= 0);
            let length = length_varint.0 as usize;
            let id: VarInt<i32> = VarInt::deserialize(input)?;
            let data = Bytes::copy_from_slice(
                input
                    .input
                    .slice(input.offset..(input.offset + length - id.length_of())),
            );
            Self { length, id, data }
        }
    }
}

impl Serialize for SerializedPacket {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), Error> {
        let length = VarInt::<i32>(self.length.try_into().map_err(|_| Error::VarIntTooBig)?);
        buf.reserve(length.length_of() + self.id.length_of() + self.data.len());
        length.serialize_to(buf)?;
        self.id.serialize_to(buf)?;
        buf.put_slice(&self.data);
        Ok(())
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
