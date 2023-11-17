#![allow(dead_code)]
use crate::{
    nbt::Nbt,
    nsfr::{i12, i26},
};
use derive_more::*;
use indexmap::IndexMap;
use miette::SourceSpan;
use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, Range},
    rc::Rc,
    sync::Arc,
};
use uuid::Uuid;

use crate::model::VarInt;
use ::bytes::{BufMut, Bytes, BytesMut};
pub use aott::prelude::parser;
pub use aott::prelude::Parser;
use aott::{iter::IterParser, prelude::*};

mod error;
mod types;

pub use error::*;
pub use types::{Label as Type, *};

pub trait Deserialize: Sized {
    type Context = ();

    fn deserialize<'a>(
        input: &mut Input<&'a [u8], Extra<Self::Context>>,
    ) -> PResult<&'a [u8], Self, Extra<Self::Context>>;
}

pub trait Serialize {
    fn serialize(&self) -> Result<::bytes::Bytes, crate::error::Error> {
        try {
            let mut b = BytesMut::new();
            self.serialize_to(&mut b)?;
            b.freeze()
        }
    }
    /// Serializes `self` to the given buffer.
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error>;
}

pub fn any_of<T: Debug>(things: &[T]) -> String {
    match things {
        [el] => format!("{el:?}"),
        elements => format!("any of {elements:?}"),
    }
}

pub fn any_of_display<T: Display>(things: &[T]) -> String {
    match things {
        [el] => format!("{el}"),
        elements => format!(
            "any of [{}]",
            elements
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        ),
    }
}

pub type Resul<'a, T, C = ()> = PResult<&'a [u8], T, Extra<C>>;

#[parser(extras = "Extra<C>")]
pub fn deser_cx<T: Deserialize<Context = ()>, C>(input: &[u8]) -> T {
    input.parse_no_context(&T::deserialize)
}

#[parser(extras = "Extra<C>")]
pub fn deser<T: Deserialize<Context = C>, C>(input: &[u8]) -> T {
    T::deserialize(input)
}

#[parser(extras = E)]
pub fn slice_till_end<'a, I: SliceInput<'a>, E: ParserExtras<I>>(input: I) -> I::Slice {
    Ok(input.input.slice_from(input.offset..))
}

impl<'a, T: Serialize + ?Sized> Serialize for &'a T {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        (**self).serialize_to(buf)
    }
}

impl<T1: Serialize, T2: Serialize> Serialize for (T1, T2) {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        self.0.serialize_to(buf)?;
        self.1.serialize_to(buf)?;
        Ok(())
    }
}

pub macro serialize($ty:ty => [$($field:ident),*$(,)?]) {
    impl crate::ser::Serialize for $ty {
        fn serialize_to(&self, buf: &mut ::bytes::BytesMut) -> Result<(), crate::error::Error> {
            try {
                $(self.$field.serialize_to(buf)?;)*
            }
        }
    }
}
pub macro deserialize($(|$context:ty|)? $ty:ty => [$($(|$cx:ty|)?$field:ident),*$(,)?]) {
    impl crate::ser::Deserialize for $ty {
        type Context = deserialize_field!($(|$context|)?);

        #[parser(extras = "crate::ser::Extra<Self::Context>")]
        fn deserialize(input: &[u8]) -> Self {
            Ok(Self {
                $($field: crate::ser::deserialize_field!($(|$cx:ty|)?$field, input)?,)*
            })
        }
    }
}

macro deserialize_field {
    (|$cx:ty| $field:ident, $input:ident) => {
        crate::ser::deser($input)
    },
    ($field:ident, $input:ident) => {
        crate::ser::deser_cx($input)
    },
    (|$context:ty|) => {
        $context
    },
    () => {
        ()
    }
}

pub macro impl_ser($(|$context:ty|)? $ty:ty => [$($(|$cx:ty|)?$field:ident),*$(,)?]) {
    crate::ser::deserialize!($(|$context|)? $ty => [$($(|$cx|)?$field,)*]);
    crate::ser::serialize!($ty => [$($field,)*]);
}

#[inline(always)]
pub fn no_context<I: InputType, O, E: ParserExtras<I>, EV: ParserExtras<I, Context = ()>>(
    parser: impl Parser<I, O, EV>,
) -> impl Fn(&mut Input<I, E>) -> Result<O, EV::Error> {
    move |input| parser.parse_with(&mut input.no_context())
}

#[inline(always)]
pub fn with_context<I: InputType, O, E: ParserExtras<I>, E2: ParserExtras<I, Context = C>, C>(
    parser: impl Parser<I, O, E2>,
    context: C,
) -> impl Fn(&mut Input<I, E>) -> Result<O, E2::Error> {
    move |input| parser.parse_with(&mut input.with_context(&context))
}
