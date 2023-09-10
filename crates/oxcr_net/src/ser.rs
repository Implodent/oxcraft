#![allow(dead_code)]
use derive_more::*;
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, Range},
    rc::Rc,
    sync::Arc,
};
use uuid::Uuid;
use ux::{i12, i26};

use crate::model::VarInt;
use ::bytes::{BufMut, BytesMut};
pub use aott::prelude::parser;
pub use aott::prelude::Parser;
use aott::{pfn_type, prelude::*};
use tracing::debug;

pub trait Deserialize: Sized {
    type Context = ();

    fn deserialize<'a>(
        input: &mut Input<&'a [u8], Extra<Self::Context>>,
    ) -> PResult<&'a [u8], Self, Extra<Self::Context>>;
}

pub trait Serialize {
    fn serialize(&self) -> ::bytes::Bytes {
        let mut b = BytesMut::new();
        self.serialize_to(&mut b);
        b.freeze()
    }
    /// Serializes `self` to the given buffer.
    fn serialize_to(&self, buf: &mut BytesMut);
}

pub struct Extra<C>(PhantomData<C>);
impl<'a, C> ParserExtras<&'a [u8]> for Extra<C> {
    type Context = C;
    type Error = crate::error::Error;
}
impl<'a> Error<&'a [u8]> for crate::error::Error {
    type Span = Range<usize>;
    fn expected_eof_found(
        span: Self::Span,
        found: aott::MaybeRef<'_, <&'a [u8] as InputType>::Token>,
    ) -> Self {
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a [u8]>>::expected_eof_found(span, found),
        )
    }
    fn expected_token_found(
        span: Self::Span,
        expected: Vec<<&'a [u8] as InputType>::Token>,
        found: aott::MaybeRef<'_, <&'a [u8] as InputType>::Token>,
    ) -> Self {
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a [u8]>>::expected_token_found(
                span, expected, found,
            ),
        )
    }
    fn unexpected_eof(
        span: Self::Span,
        expected: Option<Vec<<&'a [u8] as InputType>::Token>>,
    ) -> Self {
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a [u8]>>::unexpected_eof(span, expected),
        )
    }
}
impl<'a, C> ParserExtras<&'a str> for Extra<C> {
    type Context = C;
    type Error = crate::error::Error;
}
impl<'a> Error<&'a str> for crate::error::Error {
    type Span = Range<usize>;
    fn expected_eof_found(
        span: Self::Span,
        found: aott::MaybeRef<'_, <&'a str as InputType>::Token>,
    ) -> Self {
        Self::SerStr(
            <extra::Simple<char> as aott::error::Error<&'a str>>::expected_eof_found(span, found),
        )
    }
    fn expected_token_found(
        span: Self::Span,
        expected: Vec<<&'a str as InputType>::Token>,
        found: aott::MaybeRef<'_, <&'a str as InputType>::Token>,
    ) -> Self {
        Self::SerStr(
            <extra::Simple<char> as aott::error::Error<&'a str>>::expected_token_found(
                span, expected, found,
            ),
        )
    }
    fn unexpected_eof(
        span: Self::Span,
        expected: Option<Vec<<&'a str as InputType>::Token>>,
    ) -> Self {
        Self::SerStr(
            <extra::Simple<char> as aott::error::Error<&'a str>>::unexpected_eof(span, expected),
        )
    }
}

pub type Resul<'a, T, C = ()> = PResult<&'a [u8], T, Extra<C>>;

#[parser(extras = "Extra<C>")]
pub fn deser_cx<T: Deserialize<Context = ()>, C>(input: &[u8]) -> T {
    no_context(T::deserialize)(input)
}

pub fn no_context<
    I: InputType,
    O,
    E: ParserExtras<I, Context = ()>,
    EE: ParserExtras<I, Context = C, Error = E::Error>,
    C,
    P: Parser<I, O, E>,
>(
    parser: P,
) -> pfn_type!(I, O, EE) {
    move |input| {
        let mut inp = Input {
            offset: input.offset,
            input: input.input,
            cx: &(),
        };
        let value = parser.parse_with(&mut inp)?;
        input.offset = inp.offset;
        Ok(value)
    }
}
pub fn with_context<
    I: InputType,
    O,
    E: ParserExtras<I, Context = C2>,
    EE: ParserExtras<I, Context = C1, Error = E::Error>,
    C1,
    C2,
    P: Parser<I, O, E>,
>(
    parser: P,
    context: C2,
) -> pfn_type!(I, O, EE) {
    move |input| {
        let mut inp = Input {
            offset: input.offset,
            input: input.input,
            cx: &context,
        };
        let value = parser.parse_with(&mut inp)?;
        input.offset = inp.offset;
        Ok(value)
    }
}

pub fn seri<T: Serialize>(t: &T) -> ::bytes::Bytes {
    t.serialize()
}

#[parser(extras = E)]
pub fn slice_till_end<'a, I: SliceInput<'a>, E: ParserExtras<I>>(input: I) -> I::Slice {
    Ok(input.input.slice_from(input.offset..))
}

impl<const N: usize, Sy: Syncable> Deserialize for FixedStr<N, Sy> {
    type Context = ();

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        // get length, must be 0 <= length <= N
        let VarInt(length) = VarInt::<i32>::deserialize(input)?;
        debug!(%length, expected_length=%N, "[fixedstr] checking length");
        assert!(length >= 0);
        let length = length as usize;
        assert!(length <= N);
        let string =
            std::str::from_utf8(input.input.slice((input.offset)..(input.offset + length)))
                .expect("invalid utf8. that's a skill issue ngl");
        input.offset += length;

        // SAFETY: checked length being <= N, can unwrap_unchecked here.
        Ok(unsafe { FixedStr::from_string(string).unwrap_unchecked() })
    }
}

impl<const N: usize, Sy: Syncable> Serialize for FixedStr<N, Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let n: i32 = N.try_into().unwrap();
        let ln = VarInt(n);
        buf.reserve(ln.length_of() + N);
        ln.serialize_to(buf);
        buf.put_slice(self.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct FixedStr<const N: usize, Sy: Syncable = YesSync> {
    inner: Sy::RefC<str>,
}
impl<const N: usize, Sy: Syncable> std::fmt::Display for FixedStr<N, Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self.inner.deref(), f)
    }
}
impl<const N: usize, Sy: Syncable> FixedStr<N, Sy> {
    pub fn from_string(string: &str) -> Option<Self> {
        if string.len() > N {
            None
        } else {
            Some(Self {
                inner: Sy::refc_from_str(string),
            })
        }
    }
}

impl<const N: usize, Sy: Syncable> Deref for FixedStr<N, Sy> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.inner
    }
}

pub trait Syncable {
    const SYNC: bool;
    type RefC<T: ?Sized>: Clone + Deref<Target = T> + Sized;

    fn refc_from_str(s: &str) -> Self::RefC<str>;
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]>;
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct YesSync;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoSync;
impl Syncable for YesSync {
    const SYNC: bool = true;
    type RefC<T: ?Sized> = Arc<T>;
    fn refc_from_str(s: &str) -> Self::RefC<str> {
        Arc::from(s)
    }
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]> {
        Arc::from(slice)
    }
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]> {
        Arc::from_iter(iter)
    }
}
impl Syncable for NoSync {
    const SYNC: bool = false;
    type RefC<T: ?Sized> = Rc<T>;
    fn refc_from_str(s: &str) -> Self::RefC<str> {
        Rc::from(s)
    }
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]> {
        Rc::from(slice)
    }
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]> {
        Rc::from_iter(iter)
    }
}

impl<'a> Serialize for &'a str {
    fn serialize_to(&self, buf: &mut BytesMut) {
        VarInt::<i32>(self.len().try_into().unwrap()).serialize_to(buf);
        buf.put_slice(self.as_bytes());
    }
}

#[derive(Clone, Copy, Deref, DerefMut, Display)]
pub struct Json<T>(pub T);
impl<T: Debug + serde::Serialize> Debug for Json<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Json({:?}) = {}",
            self.0,
            serde_json::to_string_pretty(&self.0).unwrap()
        )
    }
}

impl<T: serde::Serialize> Serialize for Json<T> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let s = serde_json::to_string(&self.0).expect("json fail");
        s.as_str().serialize_to(buf);
    }
}

impl<T: for<'de> serde::Deserialize<'de>> Deserialize for Json<T> {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let VarInt(length) = VarInt::<i32>::deserialize(input)?;
        assert!(length >= 0);
        let length = length as usize;
        let slic = input.input.slice(input.offset..input.offset + length);
        match serde_json::from_slice::<T>(slic) {
            Ok(j) => Ok(Self(j)),
            Err(e) => Err(crate::error::Error::Json(e)),
        }
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    type Context = T::Context;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let exists = one_of([0x0, 0x1]).map(|t| t == 0x1).parse_with(input)?;
        if exists {
            let value = T::deserialize(input)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

impl<T: Serialize> Serialize for Option<T> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        match self {
            Some(val) => {
                buf.put_u8(0x1);
                val.serialize_to(buf)
            }
            None => buf.put_u8(0x0),
        }
    }
}

impl Serialize for Uuid {
    fn serialize_to(&self, buf: &mut BytesMut) {
        buf.put_u128(self.as_u128())
    }
}

impl Deserialize for Uuid {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        const AMOUNT: usize = 16;

        if input.input.len() < input.offset + AMOUNT {
            let e = Error::<&'a [u8]>::unexpected_eof(input.span_since(input.offset), None);
            return Err(e);
        }

        let bytes = input.input.slice(input.offset..input.offset + AMOUNT);
        input.offset += AMOUNT;

        Ok(Self::from_slice(bytes).unwrap())
    }
}

#[derive(Debug, Deref, Clone, PartialEq, Eq)]
#[deref(forward)]
pub struct Array<T: Clone, Sy: Syncable = YesSync>(Sy::RefC<[T]>);

impl<T: Clone, Sy: Syncable> Array<T, Sy> {
    pub fn empty() -> Self {
        Self(Sy::refc_from_slice(&[]))
    }
    pub fn new(slice: &[T]) -> Self {
        Self(Sy::refc_from_slice(slice))
    }
}

impl<T: Clone + Serialize, Sy: Syncable> Serialize for Array<T, Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        VarInt::<i32>(self.len().try_into().unwrap()).serialize_to(buf);
        for item in self.iter() {
            item.serialize_to(buf);
        }
    }
}

impl<T: Clone, Sy: Syncable> FromIterator<T> for Array<T, Sy> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Sy::refc_from_iter(iter))
    }
}

impl<T: Clone + Deserialize, Sy: Syncable> Deserialize for Array<T, Sy> {
    type Context = <T as Deserialize>::Context;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let VarInt::<i32>(length) = deser_cx(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        T::deserialize
            .repeated_custom::<Self>()
            .exactly(length)
            .parse_with(input)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Identifier<Sy: Syncable = YesSync>(pub Namespace, pub Sy::RefC<str>);
impl<Sy: Syncable> std::fmt::Debug for Identifier<Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Identifier({:?}, {:?})", self.0, self.1.deref())
    }
}
impl<Sy: Syncable> Display for Identifier<Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.0, self.1.deref())
    }
}

impl<Sy: Syncable> Identifier<Sy> {
    pub fn new(namespace: Namespace, value: &str) -> Self {
        Self(namespace, Sy::refc_from_str(value))
    }

    #[parser(extras = "Extra<()>")]
    fn parse(input: &str) -> Self {
        (
            aott::text::ascii::ident.map(|namespace| match namespace {
                "minecraft" => Namespace::Minecraft,
                anything_else => Namespace::Custom(Cow::Owned(anything_else.to_owned())),
            }),
            just(":").ignored(),
            aott::text::ascii::ident.map(Sy::refc_from_str),
        )
            .map(|(namespace, (), value)| Self(namespace, value))
            .parse_with(input)
    }
}

impl<Sy: Syncable> Serialize for Identifier<Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let formatted = format!("{}:{}", self.0, &*self.1);
        formatted.as_str().serialize_to(buf)
    }
}
impl<Sy: Syncable> Deserialize for Identifier<Sy> {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let FixedStr::<32767, Sy> { inner } = FixedStr::deserialize(input)?;
        Self::parse.parse(inner.deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum Namespace {
    #[display(fmt = "minecraft")]
    Minecraft,
    #[display(fmt = "{_0}")]
    Custom(Cow<'static, str>),
}

pub trait Endian {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Big;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Little;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Native;

impl Endian for Big {}
impl Endian for Little {}
impl Endian for Native {}

/// A type for serializing and deserializing numbers from bytes.
/// # Examples
/// ```
/// # use aott::prelude::*;
/// # use aott::ser::*;
/// assert_eq!(deserialize_from::<_, extra::Err<&[u8]>, _>(&[0x63][..]), Ok(Number(99u8, Big)));
/// ```
#[derive(Copy, Clone, Deref, DerefMut, Debug, Display, PartialEq, Eq)]
#[display(bound = "N: core::fmt::Display")]
#[display(fmt = "{_0}")]
pub struct Number<N, E: Endian>(
    #[deref]
    #[deref_mut]
    pub N,
    pub E,
);

macro_rules! number_impl {
        ($($num:ty)*) => {
                $(
                impl Deserialize
                        for Number<$num, Big>
                {
                        fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                        where
                                Self: Sized,
                        {
                                Ok(Self(
                                        <$num>::from_be_bytes(
                                                take_exact::<{ core::mem::size_of::<$num>() }>()
                                                        .parse_with(input)?,
                                        ),
                                        Big,
                                ))
                        }
                }
                impl Deserialize
                        for Number<$num, Little>
                {
                        fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                        where
                                Self: Sized,
                        {
                                Ok(Self(
                                        <$num>::from_le_bytes(
                                                take_exact::<{ core::mem::size_of::<$num>() }>()
                                                        .parse_with(input)?,
                                        ),
                                        Little,
                                ))
                        }
                }
                impl Deserialize for $num {
                    fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                    where
                            Self: Sized,
                    {
                            Ok(
                                    <$num>::from_be_bytes(
                                            take_exact::<{ core::mem::size_of::<$num>() }>()
                                                    .parse_with(input)?,
                                    ),
                            )
                    }
                }
                impl Serialize for $num {
                    fn serialize_to(&self, buf: &mut BytesMut) {
                        buf.put(&self.to_be_bytes()[..])
                    }
                }
            )*
        };
}

number_impl![u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64];

/// # Layout
/// x as a 26-bit integer, followed by z as a 26-bit integer, followed by y as a 12-bit integer (all signed, two's complement).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub x: i26,
    pub y: i12,
    pub z: i26,
}

pub macro serialize($ty:ty => [$(!)?$($field:ident),*$(,)?]) {
    impl crate::ser::Serialize for $ty {
        fn serialize_to(&self, buf: &mut ::bytes::BytesMut) {
            $(self.$field.serialize_to(buf);)*
        }
    }
}
pub macro deserialize($(|$context:ty|)? $ty:ty => [$(!)?$($field:ident),*$(,)?]) {
    impl crate::ser::Deserialize for $ty {
        type Context = deserialize_field!($(|$context|)?);

        #[parser(extras = "crate::ser::Extra<Self::Context>")]
        fn deserialize(input: &[u8]) -> Self {
            Ok(Self {
                $($field: crate::ser::deserialize_field!($(!)?$field)?,)*
            })
        }
    }
}

pub macro impl_ser($(|$context:ty|)? $ty:ty => [$(!)?$($field:ident),*$(,)?]) {
    serialize!($ty => [$(!)?$($field),*$(,)?]);
    deserialize!($(|$context|)? $ty => [$(!)?$($field),*$(,)?]);
}

macro deserialize_field {
    (!$field:ident) => {
        $field: deser_cx(input)?
    },
    ($field:ident) => {
        $field: deser(input)?
    },
    (|$context:ty|) => {
        $context
    },
    () => {
        ()
    }
}
