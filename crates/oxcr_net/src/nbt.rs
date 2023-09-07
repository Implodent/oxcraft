#![allow(dead_code)]
use aott::{
    bytes::{self as b, number::big},
    primitive::{any, take},
};
use bytes::BufMut;
use derive_more::*;
use std::collections::HashMap;

use crate::{explode, ser::*};

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
    fn single(input: &[u8]) -> Option<Self> {
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
                NbtTag::Normal(NbtNormal { name, value, .. }) => map.insert(name, value),
                NbtTag::List(_) => {
                    #[cfg(not(debug_assertions))]
                    unsafe {
                        std::hint::unreachable_unchecked()
                    }
                    #[cfg(debug_assertions)]
                    unreachable!("encountered nbt list as element of nbt compound");
                }
            };
        }

        Ok(map)
    }
}

struct NbtNormal {
    pub tag: NbtTagType,
    pub name: String,
    pub value: Nbt,
}
struct NbtList {
    pub tag: NbtTagType,
    // must fit in an i32, else ub
    pub length: usize,
    pub tags: Vec<Nbt>,
}

enum NbtTag {
    End,
    Normal(NbtNormal),
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

        Ok(Self::Normal(NbtNormal {
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
            explode!();
        }

        let length = b::number::big::i32(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        let tags = with_context(
            Nbt::single
                .map(|v| v.unwrap_or_else(|| explode!()))
                .repeated()
                .exactly(length),
            tag,
        )(input)?;
        Ok(NbtList { tag, length, tags })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NbtTagType {
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
struct SmolArray<T>(pub Vec<T>);

impl<T> SmolArray<T> {
    pub fn new(vec: Vec<T>) -> Self {
        Self(vec)
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
        buf.put_i32(self.len().try_into().unwrap());
        for item in self.iter() {
            item.serialize_to(buf);
        }
    }
}
