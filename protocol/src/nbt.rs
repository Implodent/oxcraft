#![allow(dead_code)]

use aott::{
    bytes::{self as b, number::big},
    primitive::{any, take},
};
use bytes::{BufMut, BytesMut};
use derive_more::*;
use serde::{
    de::{MapAccess, SeqAccess},
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Deserializer, Serializer,
};
use std::collections::HashMap;

use crate::ser::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Nbt {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<Nbt>),
    Compound(HashMap<String, Nbt>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Nbt {
    #[parser(extras = "Extra<NbtTagType>")]
    #[doc(hidden)]
    pub fn single(input: &[u8]) -> Option<Self> {
        Ok(Some(match input.context() {
            NbtTagType::End => return Ok(None),
            NbtTagType::Byte => Nbt::Byte(big::i8(input)?),
            NbtTagType::Short => Nbt::Short(big::i16(input)?),
            NbtTagType::Int => Nbt::Int(big::i32(input)?),
            NbtTagType::Long => Nbt::Long(big::i64(input)?),
            NbtTagType::Float => Nbt::Float(big::f32(input)?),
            NbtTagType::Double => Nbt::Double(big::f64(input)?),
            NbtTagType::ByteArray => Nbt::ByteArray(deser_cx::<SmolArray<i8>, _>(input)?.0),
            NbtTagType::IntArray => Nbt::IntArray(deser_cx::<SmolArray<i32>, _>(input)?.0),
            NbtTagType::LongArray => Nbt::LongArray(deser_cx::<SmolArray<i64>, _>(input)?.0),
            NbtTagType::List => Self::List(no_context(NbtTag::list)(input)?.tags),
            NbtTagType::Compound => Self::Compound(no_context(Self::compound)(input)?),
            NbtTagType::String => Self::String(no_context(nbt_string)(input)?),
        }))
    }

    #[parser(extras = "Extra<()>")]
    pub fn compound(input: &[u8]) -> HashMap<String, Self> {
        let mut map = HashMap::new();

        loop {
            match NbtTag::named(input)? {
                NbtTag::End => break,
                NbtTag::Named(NbtNamed { name, value, .. }) => map.insert(name, value),
                NbtTag::List(NbtList { tags, .. }) => {
                    return Err(crate::error::Error::Nbt(NbtError::Expected {
                        expected: NbtExpected::NamedTag,
                        actual: Nbt::List(tags),
                    }))
                }
            };
        }

        Ok(map)
    }

    fn tag(&self) -> NbtTagType {
        use NbtTagType as T;
        match &self {
            Self::Byte(_) => T::Byte,
            Self::ByteArray(_) => T::ByteArray,
            Self::Compound(_) => T::Compound,
            Self::Double(_) => T::Double,
            Self::Float(_) => T::Float,
            Self::Int(_) => T::Int,
            Self::IntArray(_) => T::IntArray,
            Self::List(_) => T::List,
            Self::Long(_) => T::Long,
            Self::LongArray(_) => T::LongArray,
            Self::Short(_) => T::Short,
            Self::String(_) => T::String,
        }
    }

    #[inline(always)]
    pub fn serialize_value(&self, buf: &mut bytes::BytesMut) {
        match self {
            Self::Byte(b) => buf.put_i8(*b),
            Self::ByteArray(ba) => SmolArray::serialize_slice(ba, buf),
            Self::Compound(compound) => Self::serialize_compound(compound, buf),
            Self::Double(d) => buf.put_f64(*d),
            Self::Float(f) => buf.put_f32(*f),
            Self::Int(i) => buf.put_i32(*i),
            Self::IntArray(ia) => SmolArray::serialize_slice(ia, buf),
            Self::List(list) => Self::serialize_list(list, buf, cfg!(debug_assertions)),
            Self::Long(lg) => buf.put_i64(*lg),
            Self::LongArray(la) => SmolArray::serialize_slice(la, buf),
            Self::Short(s) => buf.put_i16(*s),
            Self::String(s) => {
                buf.put_u16(s.len().try_into().expect("string length was more than u16"));
                buf.put_slice(s.as_bytes())
            }
        }
    }

    #[inline(always)]
    pub fn serialize_compound(compound: &HashMap<String, Self>, buf: &mut bytes::BytesMut) {
        for (name, value) in compound {
            NbtNamed::serialize_named(value.tag(), name, value, buf)
        }
        buf.put_u8(NbtTagType::End as _);
    }

    #[inline(always)]
    pub fn serialize_list(list: &[Self], buf: &mut bytes::BytesMut, check_type: bool) {
        let r: Result<(), NbtTagType> = try {
            let tag = match list.first() {
                None => return,
                Some(s) => s,
            }
            .tag();

            buf.put_u8(tag as _);
            buf.put_i32(list.len().try_into().expect("nbt list length exceeded i32"));

            for item in list.iter() {
                if check_type && item.tag() != tag {
                    Err(item.tag())?;
                }

                item.serialize_value(buf);
            }
        };

        r.unwrap_or_else(|t| panic!("type-check failed: {t:?}"))
    }
}

impl Serialize for [Nbt] {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        Nbt::serialize_list(self, buf, cfg!(debug_assertions))
    }
}

impl Serialize for Nbt {
    fn serialize_to(&self, buf: &mut BytesMut) {
        self.serialize_value(buf);
    }
}

impl<'de> Deserialize for Nbt {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let tag = with_context(Nbt::single, NbtTagType::Compound)(input)?;
        todo!()
    }
}

struct NbtList {
    pub tag: NbtTagType,
    pub length: usize,
    pub tags: Vec<Nbt>,
}

struct NbtNamed {
    pub tag: NbtTagType,
    pub name: String,
    pub value: Nbt,
}

impl NbtNamed {
    pub fn serialize_named(tag: NbtTagType, name: &str, value: &Nbt, buf: &mut BytesMut) {
        // typeid
        buf.put_u8(tag as _);

        // name
        buf.put_u16(name.len().try_into().expect("usize > u16"));
        buf.put_slice(name.as_bytes());

        value.serialize_value(buf);
    }
}

impl Serialize for NbtNamed {
    fn serialize_to(&self, buf: &mut BytesMut) {
        Self::serialize_named(self.tag, &self.name, &self.value, buf);
    }
}

enum NbtTag {
    End,
    Named(NbtNamed),
    List(NbtList),
}

impl NbtTag {
    #[parser(extras = "Extra<()>")]
    pub fn named(input: &[u8]) -> Self {
        let tag = nbt_tag(input)?;

        if tag == NbtTagType::End {
            return Ok(Self::End);
        }

        let name = nbt_string(input)?;

        Ok(Self::Named(NbtNamed {
            tag,
            name,
            // SAFETY: end tag type was handled beforehand, so we can safely explode here
            value: unsafe { with_context(Nbt::single, tag)(input)?.unwrap_unchecked() },
        }))
    }

    #[parser(extras = "Extra<()>")]
    pub fn list(input: &[u8]) -> NbtList {
        let tag = nbt_tag(input)?;

        if tag == NbtTagType::End {
            return Err(crate::error::Error::Nbt(NbtError::ExpectedAnythingButEnd));
        }

        let length = b::number::big::i32(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        let tags = with_context(
            Nbt::single
                .try_map(|v: Option<Nbt>| {
                    v.ok_or(crate::error::Error::Nbt(NbtError::ExpectedAnythingButEnd))
                })
                .repeated()
                .exactly(length),
            tag,
        )(input)?;

        Ok(NbtList { tag, length, tags })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum NbtTagType {
    End = 0,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    List = 9,
    Compound = 10,
    IntArray,
    LongArray = 12,
}

#[parser(extras = "Extra<()>")]
fn nbt_tag(input: &[u8]) -> NbtTagType {
    any.filter(|b| (0u8..=12u8).contains(b))
        // SAFETY: in filter we filter the tag types to be in bounds
        .map(|b| unsafe { *(&b as *const u8 as *const NbtTagType) })
        .parse_with(input)
}

#[parser(extras = "Extra<()>")]
fn nbt_string(input: &[u8]) -> String {
    let length = b::number::big::u16(input)? as usize;
    let s = take(length)(input)?;

    Ok(String::from_utf8(s)?)
}

#[derive(Debug, Clone, Deref)]
#[deref(forward)]
#[repr(transparent)]
struct SmolArray<T>(pub Vec<T>);

impl<T> SmolArray<T> {
    pub fn new(vec: Vec<T>) -> Self {
        Self(vec)
    }
    pub fn serialize_slice(slc: &[T], buf: &mut bytes::BytesMut)
    where
        T: Serialize,
    {
        buf.put_i32(slc.len().try_into().unwrap());
        for item in slc.iter() {
            item.serialize_to(buf);
        }
    }
}

impl<T: Deserialize> Deserialize for SmolArray<T> {
    type Context = T::Context;
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let length = b::number::big::i32(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        T::deserialize
            .repeated_custom::<Self>()
            .exactly(length)
            .parse_with(input)
    }
}

impl<A> FromIterator<A> for SmolArray<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Serialize> Serialize for SmolArray<T> {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        Self::serialize_slice(&self.0, buf)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NbtSerde<T>(pub T);

impl<T: serde::Serialize> Serialize for NbtSerde<T> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let r: Result<(), crate::error::Error> = try { nbt_serde(&self.0)?.serialize_value(buf) };
        r.unwrap()
    }
}

impl<T: for<'de> serde::de::Deserialize<'de>> Deserialize for NbtSerde<T> {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let Some(tag) = with_context(Nbt::single, NbtTagType::Compound)(input)? else {
            return Err(NbtError::ExpectedAnythingButEnd.into());
        };

        T::deserialize(NbtDe { input: &tag })
            .map(Self)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nbt(value: Nbt, bytes: &[u8]) {
        let mut buf = BytesMut::new();
        value.serialize_value(&mut buf);
        let buff = &buf[..];
        eprintln!("comparing {value:?}\n left = {buff:x?}\nright = {bytes:x?}",);
        assert_eq!(buff, bytes);
    }

    fn test_nbt_named(name: &str, value: Nbt, bytes: &[u8]) {
        let mut buf = BytesMut::new();
        NbtNamed::serialize_named(value.tag(), name, &value, &mut buf);
        let buff = &buf[..];
        eprintln!("comparing {name} {value:?}\n left = {buff:x?}\nright = {bytes:x?}",);
        assert_eq!(buff, bytes);
    }

    fn test_nbt_json<T: serde::Serialize + std::fmt::Debug>(json: T, bytes: &[u8]) {
        eprintln!("testing {json:?}");
        let nj = NbtSer(json);
        let buf = nj.serialize();
        let buff = &buf[..];
        eprintln!(" left = {buff:x?}\nright = {bytes:x?}");
        assert_eq!(buff, bytes);
    }

    #[test]
    fn named_tests() {
        test_nbt_named(
            "shortTest",
            Nbt::Short(32767),
            &[
                0x02, // type id
                0x00, 0x09, // length of name
                0x73, 0x68, 0x6f, 0x72, 0x74, 0x54, 0x65, 0x73, 0x74, // name
                0x7f, 0xff, // payload
            ],
        );
        test_nbt_named(
            "hello world",
            Nbt::Compound(
                [("name".to_string(), Nbt::String("Bananrama".to_string()))]
                    .into_iter()
                    .collect(),
            ),
            &[
                0x0a, // type id of the root compound (0x0a, duh)
                0x00, 0x0b, // length of name of the root compound
                0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c,
                0x64, // name of the root compound
                0x08, // type id of first element
                0x00, 0x04, // length of name of first element (4)
                0x6e, 0x61, 0x6d, 0x65, // name of first element ("name")
                0x00, 0x09, // length of string named "name" (9)
                0x42, 0x61, 0x6e, 0x61, 0x6e, 0x72, 0x61, 0x6d, 0x61, // string ("Bananrama")
                0x00, // TAG_End
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    fn normal_tests() {
        {
            let string = "uqwjmorpqiwuechrqweirwqeфщцшуйзцшуй";
            let mut bytes = BytesMut::new();
            bytes.put_u16(string.len() as u16);
            bytes.put_slice(string.as_bytes());
            test_nbt(Nbt::String(string.to_string()), &bytes[..]);
        }
        {
            let number: i16 = 0x70;
            test_nbt(Nbt::Short(number), &[0x00, 0x70                                    ]);
        }
        {
            let number: i32 = 0xd7fa8be;
            test_nbt(Nbt::Int  (number), &[0x0d, 0x7f, 0xa8, 0xbe                        ]);
        }
        {
            let number: i64 = 0xf7ba6cf39efb2e5;
            test_nbt(Nbt::Long (number), &[0x0f, 0x7b, 0xa6, 0xcf, 0x39, 0xef, 0xb2, 0xe5]);
        }
        {
            let list = vec![Nbt::Int(0xfeedbee), Nbt::Int(0xfcafeba), Nbt::Int(0xbe00000)];
            test_nbt(Nbt::List(list), &[NbtTagType::Int as u8, 0x0, 0x0, 0x0, 0x3, 0xf, 0xee, 0xdb, 0xee, 0x0f, 0xca, 0xfe, 0xba, 0xb, 0xe0, 0x00, 0x00])
        }
    }
}

struct NbtSer;

impl Serializer for NbtSer {
    type Ok = Nbt;
    type Error = NbtError;
    type SerializeMap = NbtSerMap;
    type SerializeSeq = NbtSerSeq;
    type SerializeStruct = NbtSerMap;
    type SerializeStructVariant = NbtSerMap;
    type SerializeTuple = NbtSerSeq;
    type SerializeTupleStruct = NbtSerSeq;
    type SerializeTupleVariant = NbtSerSeq;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Byte(v as i8))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::ByteArray(
            v.into_iter()
                .map(|b| {
                    (*b).try_into().map_err(|_| NbtError::OutOfBounds {
                        value: format!("{b}u8"),
                        actual_type: "u8",
                        type_for_nbt: "i8",
                    })
                })
                .try_collect()?,
        ))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::String(String::from(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Double(v))
    }

    fn serialize_i128(self, _unsupported: i128) -> Result<Self::Ok, Self::Error> {
        Err(NbtError::UnsupportedType("i128", Some("NBT does not have a 128-bit number type. Store it as 2 longs - 64 least significant bits, and 64 most significant bits.")))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Short(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Long(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Byte(v))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(NbtSerMap(
            len.map(HashMap::with_capacity).unwrap_or_else(HashMap::new),
            None,
            None,
        ))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Ok(Nbt::Compound(HashMap::from_iter([(
            variant.to_string(),
            value.serialize(self)?,
        )])))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(NbtError::UnsupportedType(
            "null",
            Some("NBT does not support nulls"),
        ))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(NbtSerSeq(
            len.map(Vec::with_capacity).unwrap_or_else(Vec::new),
            None,
        ))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::String(v.to_string()))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(NbtSerMap(HashMap::with_capacity(len), None, None))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(NbtSerMap(HashMap::with_capacity(len), Some(variant), None))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(NbtSerSeq(Vec::with_capacity(len), None))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(NbtSerSeq(Vec::with_capacity(len), None))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(NbtSerSeq(Vec::with_capacity(len), Some(variant)))
    }

    fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        K: serde::Serialize,
        V: serde::Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        Ok(Nbt::Compound(
            iter.into_iter()
                .map(|(key, value)| match key.serialize(NbtSer)? {
                    Nbt::String(s) => Ok((s, value.serialize(NbtSer)?)),
                    actual_key => Err(NbtError::InvalidKey(actual_key)),
                })
                .try_collect()?,
        ))
    }

    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: serde::Serialize,
    {
        iter.into_iter()
            .map(|thing| serde::Serialize::serialize(&thing, NbtSer))
            .try_collect::<Vec<Nbt>>()
            .map(Nbt::List)
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display,
    {
        Ok(Nbt::String(value.to_string()))
    }

    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        Err(NbtError::UnsupportedType("u128", Some("NBT does not support unsigned integers, and even 128-bit numbers. Store it as 2 longs - 64 LSBs, and 64 MSBs.")))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.try_into()
            .map_err(|_| NbtError::OutOfBounds {
                value: format!("{v}u8"),
                actual_type: "u8",
                type_for_nbt: "i8",
            })
            .map(Nbt::Byte)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.try_into()
            .map_err(|_| NbtError::OutOfBounds {
                value: format!("{v}u16"),
                actual_type: "u16",
                type_for_nbt: "i16",
            })
            .map(Nbt::Short)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.try_into()
            .map_err(|_| NbtError::OutOfBounds {
                value: format!("{v}u32"),
                actual_type: "u32",
                type_for_nbt: "i32",
            })
            .map(Nbt::Int)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.try_into()
            .map_err(|_| NbtError::OutOfBounds {
                value: format!("{v}u64"),
                actual_type: "u64",
                type_for_nbt: "i64",
            })
            .map(Nbt::Long)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(NbtError::UnsupportedType(
            "()",
            Some("NBT does not support nulls, nor unit types."),
        ))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit_struct(_name)
    }
}

struct NbtDe<'de> {
    input: &'de Nbt,
}

struct NbtDeMap<'de>(
    std::collections::hash_map::Iter<'de, String, Nbt>,
    Option<(&'de String, &'de Nbt)>,
);

impl<'de> MapAccess<'de> for NbtDeMap<'de> {
    type Error = NbtError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let Some((next_key, next_value)) = self.0.next() else {
            return Ok(None);
        };
        self.1.replace((next_key, next_value));
        seed.deserialize(serde::de::value::StrDeserializer::new(next_key.as_str()))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let Some((_next_key, next_value)) = self.1.take() else {
            return Err(NbtError::NoKeyInMap);
        };

        seed.deserialize(NbtDe { input: next_value })
    }

    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
        V: serde::de::DeserializeSeed<'de>,
    {
        let Some((next_key, next_value)) = self.0.next() else {
            return Ok(None);
        };

        Ok(Some((
            kseed.deserialize(serde::de::value::StrDeserializer::new(next_key.as_str()))?,
            vseed.deserialize(NbtDe { input: next_value })?,
        )))
    }
}

macro_rules! declare_nbt_array_deserializer {
    ($name:ident, $v:ty, $valty:ident) => {
        struct $name<'de>(std::slice::Iter<'de, $v>, &'de [$v]);

        impl<'de> SeqAccess<'de> for $name<'de> {
            type Error = NbtError;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                let Some(value) = self.0.next() else {
                    return Ok(None);
                };

                Ok(Some(seed.deserialize(serde::de::value::$valty::<
                    Self::Error,
                >::new(*value))?))
            }

            fn size_hint(&self) -> Option<usize> {
                Some(self.1.len())
            }
        }
    };
}

declare_nbt_array_deserializer!(NbtDeByteArray, i8, I8Deserializer);
declare_nbt_array_deserializer!(NbtDeIntArray, i32, I32Deserializer);
declare_nbt_array_deserializer!(NbtDeLongArray, i64, I64Deserializer);

struct NbtDeList<'de>(std::slice::Iter<'de, Nbt>, &'de [Nbt]);

impl<'de> SeqAccess<'de> for NbtDeList<'de> {
    type Error = NbtError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        let Some(value) = self.0.next() else {
            return Ok(None);
        };

        Ok(Some(seed.deserialize(NbtDe { input: value })?))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.1.len())
    }
}

impl<'de> Deserializer<'de> for NbtDe<'de> {
    type Error = NbtError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Byte(byte) => visitor.visit_i8(*byte),
            Nbt::Short(short) => visitor.visit_i16(*short),
            Nbt::Int(int) => visitor.visit_i32(*int),
            Nbt::Long(long) => visitor.visit_i64(*long),
            Nbt::Float(float) => visitor.visit_f32(*float),
            Nbt::Double(double) => visitor.visit_f64(*double),
            Nbt::ByteArray(bytes) => visitor.visit_seq(NbtDeByteArray(bytes.iter(), &bytes[..])),
            Nbt::String(s) => visitor.visit_borrowed_str(s.as_str()),
            Nbt::List(list) => visitor.visit_seq(NbtDeList(list.iter(), &list[..])),
            Nbt::Compound(compound) => visitor.visit_map(NbtDeMap(compound.iter(), None)),
            Nbt::IntArray(int_array) => {
                visitor.visit_seq(NbtDeIntArray(int_array.iter(), &int_array[..]))
            }
            Nbt::LongArray(long_array) => {
                visitor.visit_seq(NbtDeLongArray(long_array.iter(), &long_array[..]))
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bool(match self.input {
            Nbt::Byte(0) => false,
            Nbt::Byte(1) => true,
            actual => {
                return Err(NbtError::Expected {
                    expected: NbtExpected::AnyOf(vec![Nbt::Byte(0), Nbt::Byte(1)]),
                    actual: actual.clone(),
                })
            }
        })
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Byte(byte) = self.input {
            visitor.visit_i8(*byte)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Byte),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Short(short) = self.input {
            visitor.visit_i16(*short)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Short),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Int(int) = self.input {
            visitor.visit_i32(*int)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Int),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Long(long) = self.input {
            visitor.visit_i64(*long)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Long),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Byte(byte) if *byte >= 0 => visitor.visit_u8(*byte as u8),
            actual => Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Byte),
                actual: actual.clone(),
            }),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Short(short) if *short >= 0 => visitor.visit_u16(*short as u16),
            actual => Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Short),
                actual: actual.clone(),
            }),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Int(int) if *int >= 0 => visitor.visit_u32(*int as u32),
            actual => Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Int),
                actual: actual.clone(),
            }),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Long(long) if *long >= 0 => visitor.visit_u64(*long as u64),
            actual => Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Long),
                actual: actual.clone(),
            }),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Float(float) = self.input {
            visitor.visit_f32(*float)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Float),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Double(double) = self.input {
            visitor.visit_f64(*double)
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Double),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.input {
            Nbt::Int(int) if *int >= 0 => visitor.visit_char((*int as u32).try_into().map_err(
                |_| NbtError::OutOfBounds {
                    value: format!("{int}i32"),
                    actual_type: "i32",
                    type_for_nbt: "char",
                },
            )?),
            actual => Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Int),
                actual: actual.clone(),
            }),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::String(ref string) = self.input {
            visitor.visit_str(string.as_str())
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::String),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::String(string) = self.input {
            visitor.visit_string(string.clone())
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::String),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(NbtError::UnsupportedType(
            "&[u8]",
            Some("NBT does not have an unsigned byte type"),
        ))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(NbtError::UnsupportedType(
            "Vec<u8>",
            Some("NBT does not have an unsigned byte type"),
        ))
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(NbtError::UnsupportedType("Option", Some("NBT does not suport nulls, so just use `#[serde(skip_serializing_if = \"Option::is_none\")]`")))
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(NbtError::UnsupportedType(
            "()",
            Some("NBT does not support nulls nor unit types."),
        ))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::List(list) = self.input {
            visitor.visit_seq(NbtDeList(list.iter(), list))
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::List),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if let Nbt::Compound(compound) = self.input {
            visitor.visit_map(NbtDeMap(compound.iter(), None))
        } else {
            Err(NbtError::Expected {
                expected: NbtExpected::Type(NbtTagType::Compound),
                actual: self.input.clone(),
            })
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(NbtError::UnsupportedType(
            "IgnoredAny",
            Some("NBT does not support nulls and IgnoredAny is confusing aaaaa help please"),
        ))
    }
}

impl serde::ser::Error for NbtError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        NbtError::SerdeCustom(msg.to_string())
    }
}

impl serde::de::Error for NbtError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        NbtError::SerdeCustom(msg.to_string())
    }
}

struct NbtSerMap(HashMap<String, Nbt>, Option<&'static str>, Option<String>);

impl SerializeMap for NbtSerMap {
    type Ok = Nbt;
    type Error = NbtError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let serialized_key = key.serialize(NbtSer)?;
        match serialized_key {
            Nbt::String(s) => Ok(self.2.replace(s).map(|_| ()).unwrap_or(())),
            actual_key => Err(NbtError::InvalidKey(actual_key)),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0.insert(
            self.2.take().ok_or(NbtError::NoKeyInMap)?,
            value.serialize(NbtSer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(
                key.to_string(),
                Nbt::Compound(self.0),
            )])),
            None => Nbt::Compound(self.0),
        })
    }
}

impl SerializeStruct for NbtSerMap {
    type Ok = Nbt;
    type Error = NbtError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0.insert(key.to_string(), value.serialize(NbtSer)?);
        Ok(())
    }

    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(
                key.to_string(),
                Nbt::Compound(self.0),
            )])),
            None => Nbt::Compound(self.0),
        })
    }
}

impl SerializeStructVariant for NbtSerMap {
    type Ok = Nbt;
    type Error = NbtError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0.insert(key.to_string(), value.serialize(NbtSer)?);
        Ok(())
    }

    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(
                key.to_string(),
                Nbt::Compound(self.0),
            )])),
            None => Nbt::Compound(self.0),
        })
    }
}

struct NbtSerSeq(Vec<Nbt>, Option<&'static str>);

impl SerializeSeq for NbtSerSeq {
    type Ok = Nbt;
    type Error = NbtError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(NbtSer).map(|v| self.0.push(v))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(key.to_string(), Nbt::List(self.0))])),
            None => Nbt::List(self.0),
        })
    }
}

impl SerializeTuple for NbtSerSeq {
    type Ok = Nbt;
    type Error = NbtError;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(key.to_string(), Nbt::List(self.0))])),
            None => Nbt::List(self.0),
        })
    }

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(NbtSer).map(|v| self.0.push(v))
    }
}

impl SerializeTupleStruct for NbtSerSeq {
    type Ok = Nbt;
    type Error = NbtError;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(key.to_string(), Nbt::List(self.0))])),
            None => Nbt::List(self.0),
        })
    }

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(NbtSer).map(|v| self.0.push(v))
    }
}

impl SerializeTupleVariant for NbtSerSeq {
    type Ok = Nbt;
    type Error = NbtError;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(key.to_string(), Nbt::List(self.0))])),
            None => Nbt::List(self.0),
        })
    }

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(NbtSer).map(|v| self.0.push(v))
    }
}

pub fn nbt_serde<T: serde::Serialize>(value: &T) -> Result<Nbt, NbtError> {
    value.serialize(NbtSer)
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum NbtError {
    #[error("Unsupported type for NBT: {_0}")]
    #[diagnostic(
        code(nbt::error::unsupported_type),
        url("https://wiki.vg/NBT#Specification")
    )]
    UnsupportedType(&'static str, #[help] Option<&'static str>),

    #[error("{_0}")]
    #[diagnostic(code(nbt::error::serde_custom_error))]
    SerdeCustom(String),

    #[error("{value} of type {actual_type} is out of bounds for NBT (tried to convert to {type_for_nbt})")]
    #[diagnostic(
        code(nbt::error::out_of_bounds),
        url("https://wiki.vg/NBT#Specification"),
        help(
            "NBT does not support unsigned types,\
              and because of that this NBT implementation converts them to signed types.\
              If an unsigned value is out of bounds for the signed type,\
              this error is returned."
        )
    )]
    OutOfBounds {
        value: String,
        actual_type: &'static str,
        type_for_nbt: &'static str,
    },

    #[error("{_0:?} is an invalid key for a NBT Compound.")]
    #[diagnostic(
        code(nbt::error::invalid_key_for_compound),
        url("https://wiki.vg/NBT#Specification"),
        help(
            "NBT Compounds are like JSON objects,\
              or a HashMap with String keys and Nbt values.\
              As such, keys for NBT Compounds can only be Strings,\
              and if a key is not a Nbt::String, this error is returned with the actual value of the key."
        )
    )]
    InvalidKey(Nbt),

    #[error("expected {expected}, actual: {actual:?}")]
    #[diagnostic(code(nbt::error::expected))]
    Expected { expected: NbtExpected, actual: Nbt },

    #[error("a call to serialize_value (when serializing a map) was not preceded by a call to serialize_key")]
    #[diagnostic(
        code(nbt::error::no_key_in_map),
        help("call serialize_key before calling serialize_value")
    )]
    NoKeyInMap,

    #[error("expected **anything** but TAG_End")]
    #[diagnostic(code(nbt::error::anything_but_end))]
    ExpectedAnythingButEnd,
}

#[derive(thiserror::Error, Debug)]
pub enum NbtExpected {
    #[error("{}", crate::ser::any_of(.0))]
    AnyOf(Vec<Nbt>),

    #[error("{_0:?}")]
    Type(NbtTagType),

    #[error("a named tag or TAG_End")]
    NamedTag,
}
