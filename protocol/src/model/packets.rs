use std::ops::Deref;

use crate::error::Error;
use crate::ser::*;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::{pfn_type, prelude::*};
use tracing::{trace, warn};

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
            .map_err(err_with_source(
                || self.data.clone(),
                Some(format!("packet_{:#x}.bin", self.id.0)),
            ))
    }

    pub fn serialize_compressing(&self, compression: Option<usize>) -> Result<Bytes, Error> {
        if let Some(cmp) = compression {
            let data_length = (self.length >= cmp)
                .then(|| self.id.length_of() + self.data.len())
                .unwrap_or(0);
            let datalength = VarInt::<i32>(data_length.try_into().unwrap());
            let length =
                datalength.length_of() + Compress((&self.id, &self.data), Zlib).serialize()?.len();
            let pack = SerializedPacketCompressed {
                length,
                data_length,
                id: self.id,
                data: self.data.clone(),
            };
            pack.serialize()
        } else {
            self.serialize()
        }
    }

    pub fn deserialize_compressing<'a>(
        compression: Option<usize>,
    ) -> pfn_type!(&'a [u8], Self, Extra<<Self as Deserialize>::Context>) {
        move |input| {
            if let Some(cmp) = compression {
                SerializedPacketCompressed::deserialize(input)
                    .map(Self::from)
                    .map(|v| {
                        if v.length < cmp {
                            warn!(
                                packet_length = v.length,
                                compresssion_threshold = cmp,
                                "Packet length was less than compression threshold"
                            );
                        }

                        v
                    })
            } else {
                Self::deserialize(input)
            }
        }
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
pub struct SerializedPacketCompressed {
    pub length: usize,
    pub data_length: usize,
    pub id: super::VarInt,
    pub data: Bytes,
}

impl SerializedPacketCompressed {
    pub fn new<P: Packet + Serialize>(packet: P) -> Result<Self, Error> {
        Self::new_ref(&packet)
    }

    pub fn new_ref<P: Packet + Serialize + ?Sized>(packet: &P) -> Result<Self, Error> {
        try {
            let data = packet.serialize()?;
            let id = P::ID;
            let data_length = id.length_of() + data.len();
            let datalength = VarInt::<i32>(data_length.try_into().unwrap());
            let length = datalength.length_of() + Compress(datalength, Zlib).serialize()?.len();
            Self {
                length,
                data_length,
                id,
                data,
            }
        }
    }

    pub fn try_deserialize<P: Packet + Deserialize<Context = PacketContext>>(
        &self,
        state: State,
    ) -> Result<P, Error> {
        let context = PacketContext { id: self.id, state };

        P::deserialize
            .parse_with_context(&self.data, context)
            .map_err(err_with_source(
                || self.data.clone(),
                Some(format!("packet_{:#x}.bin", self.id.0)),
            ))
    }
}

pub fn err_with_source(
    source: impl FnOnce() -> Bytes,
    name: Option<String>,
) -> impl FnOnce(Error) -> Error {
    move |e| match e {
        Error::Ser(error) => Error::SerSrc(WithSource {
            src: BytesSource::new(source(), name),
            errors: vec![error],
        }),
        e => e,
    }
}

impl Deserialize for SerializedPacketCompressed {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        try {
            let packet_length_varint = VarInt::<i32>::deserialize(input)?;
            assert!(packet_length_varint.0 > 0);
            let packet_length = packet_length_varint.0 as usize;
            let data_length_varint = VarInt::<i32>::deserialize(input)?;
            assert!(data_length_varint.0 >= 0);
            let data_length = data_length_varint.0 as usize;
            let actual_data_length = packet_length - data_length_varint.length_of();
            trace!(
                packet_length,
                data_length,
                actual_data_length,
                "decompressing serializedpacket"
            );
            let data_maybe = input.input.slice_from(input.offset..);
            let real_data = if data_length > 0 {
                let real_data = Zlib::decode(data_maybe)?;
                assert_eq!(real_data.len(), data_length);
                real_data
            } else {
                Bytes::copy_from_slice(data_maybe)
            };
            let data_slice = real_data.deref();
            let mut data_input = Input::new(&data_slice);
            let id = VarInt::<i32>::deserialize
                .parse_with(&mut data_input)
                .map_err(err_with_source(
                    || real_data.clone(),
                    Some(format!("packet_compressed_without_id.bin")),
                ))?;
            let data = data_input.input.slice_from(data_input.offset..);
            Self {
                length: packet_length,
                data_length,
                id,
                data: Bytes::copy_from_slice(data),
            }
        }
    }
}

impl Serialize for SerializedPacketCompressed {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), Error> {
        let length = VarInt::<i32>(self.length.try_into().map_err(|_| Error::VarIntTooBig)?);
        length.serialize_to(buf)?;
        let data_length = VarInt::<i32>(
            self.data_length
                .try_into()
                .map_err(|_| Error::VarIntTooBig)?,
        );
        data_length.serialize_to(buf)?;
        Compress((&self.id, &self.data), Zlib).serialize_to(buf)?;
        Ok(())
    }
}

impl From<SerializedPacketCompressed> for SerializedPacket {
    fn from(value: SerializedPacketCompressed) -> Self {
        Self {
            length: value.data_length,
            id: value.id,
            data: value.data,
        }
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
