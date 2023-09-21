#![allow(dead_code)]

use aott::{
    bytes::{self as b, number::big},
    primitive::{any, take},
};
use bytes::{BufMut, BytesMut};
use derive_more::*;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serializer,
};
use std::collections::HashMap;

use crate::{error::Error, explode, ser::*};

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
                NbtTag::List(_) => return Err(crate::error::Error::NbtFuckup),
            };
        }

        Ok(map)
    }

    fn jason_number(n: i64) -> Self {
        n.try_into()
            .map(Self::Byte)
            .or(n
                .try_into()
                .map(Self::Short)
                .or(n.try_into().map(Self::Int)))
            .unwrap_or_else(|_| Self::Long(n))
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
            value: with_context(Nbt::single, tag)(input)?.unwrap_or_else(
                || // SAFETY: end tag type was handled beforehand, so we can safely explode here
                                                                         explode!(),
            ),
        }))
    }

    #[parser(extras = "Extra<()>")]
    pub fn list(input: &[u8]) -> NbtList {
        let tag = nbt_tag(input)?;

        if tag == NbtTagType::End {
            return Err(crate::error::Error::NbtFuckup);
        }

        let length = b::number::big::i32(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        let tags = with_context(
            Nbt::single
                .try_map(|v: Option<Nbt>| v.ok_or(crate::error::Error::NbtFuckup))
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
pub struct NbtJson<T>(pub T);

impl<T: serde::Serialize> Serialize for NbtJson<T> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let r: Result<(), crate::error::Error> = try {
            nbt_serde(&self.0)
                .map_err(|_| Error::NbtFuckup)?
                .serialize_value(buf)
        };
        r.unwrap()
    }
}

impl<'de, T: serde::de::Deserialize<'de>> Deserialize for NbtJson<T> {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let tag = with_context(Nbt::single, NbtTagType::Compound)(input)?;
        todo!()
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
        let nj = NbtJson(json);
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

struct NbtSerde;

impl Serializer for NbtSerde {
    type Ok = Nbt;
    type Error = Void;
    type SerializeMap = NbtSerdeMap;
    type SerializeSeq = NbtSerdeSeq;
    type SerializeStruct = NbtSerdeMap;
    type SerializeStructVariant = NbtSerdeMap;
    type SerializeTuple = NbtSerdeSeq;
    type SerializeTupleStruct = NbtSerdeSeq;
    type SerializeTupleVariant = NbtSerdeSeq;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::Byte(v as i8))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Nbt::ByteArray(
            v.into_iter()
                .map(|b| (*b).try_into().map_err(|_| Void))
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
        Err(Void)
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
        Ok(NbtSerdeMap(
            len.map(HashMap::with_capacity).unwrap_or_else(HashMap::new),
            None,
            None,
        ))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
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
        Err(Void)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(NbtSerdeSeq(
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
        Ok(NbtSerdeMap(HashMap::with_capacity(len), None, None))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(NbtSerdeMap(
            HashMap::with_capacity(len),
            Some(variant),
            None,
        ))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(NbtSerdeSeq(Vec::with_capacity(len), None))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(NbtSerdeSeq(Vec::with_capacity(len), None))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(NbtSerdeSeq(Vec::with_capacity(len), Some(variant)))
    }

    fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        K: serde::Serialize,
        V: serde::Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        Ok(Nbt::Compound(
            iter.into_iter()
                .map(|(key, value)| match key.serialize(NbtSerde)? {
                    Nbt::String(s) => Ok((s, value.serialize(NbtSerde)?)),
                    _ => Err(Void),
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
            .map(|thing| serde::Serialize::serialize(&thing, NbtSerde))
            .try_collect::<Vec<Nbt>>()
            .map(Nbt::List)
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display,
    {
        Ok(Nbt::String(value.to_string()))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Err(Void)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        v.try_into().map_err(|_| Void).map(Nbt::Byte)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.try_into().map_err(|_| Void).map(Nbt::Short)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.try_into().map_err(|_| Void).map(Nbt::Int)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.try_into().map_err(|_| Void).map(Nbt::Long)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Void)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Void)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Void)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("")]
pub struct Void;

impl serde::ser::Error for Void {
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self
    }
}

struct NbtSerdeMap(HashMap<String, Nbt>, Option<&'static str>, Option<String>);

impl SerializeMap for NbtSerdeMap {
    type Ok = Nbt;
    type Error = Void;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let keyy = key.serialize(NbtSerde)?;
        match keyy {
            Nbt::String(s) => Ok(self.2.replace(s).map(|_| ()).unwrap_or(())),
            _ => Err(Void),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0
            .insert(self.2.take().ok_or(Void)?, value.serialize(NbtSerde)?);
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

impl SerializeStruct for NbtSerdeMap {
    type Ok = Nbt;
    type Error = Void;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0.insert(key.to_string(), value.serialize(NbtSerde)?);
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

impl SerializeStructVariant for NbtSerdeMap {
    type Ok = Nbt;
    type Error = Void;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.0.insert(key.to_string(), value.serialize(NbtSerde)?);
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

struct NbtSerdeSeq(Vec<Nbt>, Option<&'static str>);

impl SerializeSeq for NbtSerdeSeq {
    type Ok = Nbt;
    type Error = Void;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(NbtSerde).map(|v| self.0.push(v))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self.1 {
            Some(key) => Nbt::Compound(HashMap::from_iter([(key.to_string(), Nbt::List(self.0))])),
            None => Nbt::List(self.0),
        })
    }
}

impl SerializeTuple for NbtSerdeSeq {
    type Ok = Nbt;
    type Error = Void;

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
        value.serialize(NbtSerde).map(|v| self.0.push(v))
    }
}

impl SerializeTupleStruct for NbtSerdeSeq {
    type Ok = Nbt;
    type Error = Void;

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
        value.serialize(NbtSerde).map(|v| self.0.push(v))
    }
}

impl SerializeTupleVariant for NbtSerdeSeq {
    type Ok = Nbt;
    type Error = Void;

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
        value.serialize(NbtSerde).map(|v| self.0.push(v))
    }
}

pub fn nbt_serde<T: serde::Serialize>(value: &T) -> Result<Nbt, ()> {
    value.serialize(NbtSerde).map_err(|_| ())
}
