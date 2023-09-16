#![allow(dead_code)]

use aott::{
    bytes::{self as b, number::big},
    primitive::{any, take},
};
use bytes::{BufMut, BytesMut};
use derive_more::*;
use std::collections::HashMap;

use crate::{error::Error, explode, ser::*};

#[derive(Debug, Clone)]
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
            Self::String(s) => s.as_str().serialize_to(buf),
        }
    }

    #[inline(always)]
    pub fn serialize_compound(compound: &HashMap<String, Self>, buf: &mut bytes::BytesMut) {
        for (name, value) in compound {
            let name = name.as_str();

            // typeid
            buf.put_u8(value.tag() as _);

            // name
            name.serialize_to(buf);

            value.serialize_value(buf);
        }
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

impl Serialize for HashMap<String, Nbt> {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        Nbt::serialize_compound(self, buf)
    }
}
impl Serialize for [Nbt] {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        Nbt::serialize_list(self, buf, cfg!(debug_assertions))
    }
}

impl TryFrom<serde_json::Value> for Nbt {
    type Error = ();
    fn try_from(value: serde_json::Value) -> Result<Self, ()> {
        Ok(match value {
            serde_json::Value::Array(array) => {
                Self::List(array.into_iter().map(Self::try_from).try_collect()?)
            }
            serde_json::Value::Bool(bool) => Self::Byte(bool as i8),
            serde_json::Value::Null => return Err(()),
            serde_json::Value::Number(number) => number
                .as_i64()
                .map(Self::jason_number)
                .or(number.as_f64().map(Self::Double))
                .ok_or(())?,
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Object(obj) => Self::Compound(
                obj.into_iter()
                    .map(|(k, v)| Ok((k, Self::try_from(v)?)))
                    .try_collect()?,
            ),
        })
    }
}

impl Into<serde_json::Value> for Nbt {
    fn into(self) -> serde_json::Value {
        use serde_json::Value::*;
        match self {
            Nbt::Byte(n) => Number(serde_json::Number::from(n)),
            Nbt::Short(n) => Number(serde_json::Number::from(n)),
            Nbt::Int(n) => Number(serde_json::Number::from(n)),
            Nbt::Float(n) => Number(serde_json::Number::from_f64(n as f64).unwrap()),
            Nbt::Double(n) => Number(serde_json::Number::from_f64(n).unwrap()),
            Nbt::Long(n) => Number(serde_json::Number::from(n)),
            Nbt::ByteArray(a) => Array(
                a.into_iter()
                    .map(serde_json::Number::from)
                    .map(Number)
                    .collect(),
            ),
            Nbt::IntArray(a) => Array(
                a.into_iter()
                    .map(serde_json::Number::from)
                    .map(Number)
                    .collect(),
            ),
            Nbt::LongArray(a) => Array(
                a.into_iter()
                    .map(serde_json::Number::from)
                    .map(Number)
                    .collect(),
            ),
            Nbt::Compound(compound) => {
                Object(compound.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
            Nbt::List(list) => Array(list.into_iter().map(Self::into).collect()),
            Nbt::String(s) => String(s),
        }
    }
}

struct NbtNamed {
    pub tag: NbtTagType,
    pub name: String,
    pub value: Nbt,
}

struct NbtList {
    pub tag: NbtTagType,
    // must fit in an i32, else everything explodes
    pub length: usize,
    pub tags: Vec<Nbt>,
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
    List,
    Compound,
    IntArray,
    LongArray = 12,
}

#[parser(extras = "Extra<()>")]
fn nbt_tag(input: &[u8]) -> NbtTagType {
    any.filter(|b| (0u8..=12u8).contains(b))
        .map(|b| unsafe { *(&b as *const u8 as *const NbtTagType) })
        .parse_with(input)
}

#[parser(extras = "Extra<()>")]
fn nbt_string(input: &[u8]) -> String {
    let length = b::number::big::i32(input)?;
    debug_assert!(length >= 0);
    let length = length as usize;

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

impl<T: serde::Serialize + Clone> Serialize for NbtJson<T> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        // SERDE, FUCK YOU!
        let r: Result<(), crate::error::Error> = try {
            Nbt::try_from(serde_json::to_value(self.0.clone())?)
                .map_err(|_| Error::NbtFuckup)?
                .serialize_value(buf)
        };
        r.unwrap()
    }
}

impl<T: serde::de::DeserializeOwned> Deserialize for NbtJson<T> {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let tag = with_context(Nbt::single, NbtTagType::Compound)(input)?;
        Ok(Self(serde_json::from_value(tag.into())?))
    }
}
