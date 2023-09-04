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

use crate::model::VarInt;
use ::bytes::{BufMut, BytesMut};
use aott::prelude::*;
use tracing::debug;

pub trait Deserialize: Sized {
    type Context = ();

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context>;
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
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a str>>::expected_eof_found(span, found),
        )
    }
    fn expected_token_found(
        span: Self::Span,
        expected: Vec<<&'a str as InputType>::Token>,
        found: aott::MaybeRef<'_, <&'a str as InputType>::Token>,
    ) -> Self {
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a str>>::expected_token_found(
                span, expected, found,
            ),
        )
    }
    fn unexpected_eof(
        span: Self::Span,
        expected: Option<Vec<<&'a str as InputType>::Token>>,
    ) -> Self {
        Self::Ser(
            <extra::Simple<u8> as aott::error::Error<&'a str>>::unexpected_eof(span, expected),
        )
    }
}

pub type Inp<'parse, 'a, C = ()> = Input<'parse, &'a [u8], Extra<C>>;
pub type Resul<'parse, 'a, T, C = ()> = IResult<'parse, &'a [u8], Extra<C>, T>;

pub fn deser<'parse, 'a, T: Deserialize>(
    input: Inp<'parse, 'a, T::Context>,
) -> Resul<'parse, 'a, T, T::Context> {
    T::deserialize(input)
}

pub fn deser_cx<'parse, 'a, T: Deserialize<Context = ()>, C>(
    input: Inp<'parse, 'a, C>,
) -> Resul<'parse, 'a, T, C> {
    let cx = input.context();
    let (inp, res) = T::deserialize(Input {
        offset: input.offset,
        input: input.input,
        cx: &(),
    })
    .map_or_else(|(inp, e)| (inp, Err(e)), |(inp, ok)| (inp, Ok(ok)));
    let next_inp = Input {
        offset: inp.offset,
        input: inp.input,
        cx,
    };
    match res {
        Ok(ok) => Ok((next_inp, ok)),
        Err(e) => Err((next_inp, e)),
    }
}

pub fn seri<T: Serialize>(t: &T) -> ::bytes::Bytes {
    t.serialize()
}

#[parser(extras = E)]
pub fn slice_till_end<'a, I: SliceInput<'a>, E: ParserExtras<I>>(input: I) -> I::Slice {
    let slice = input.input.slice_from(input.offset..);
    Ok((input, slice))
}

impl<const N: usize, Sy: Syncable> Deserialize for FixedStr<N, Sy> {
    type Context = ();

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        // get length, must be 0 <= length <= N
        let (mut input, VarInt(length)) = VarInt::<i32>::deserialize(input)?;
        debug!(%length, expected_length=%N, "[fixedstr] checking length");
        assert!(length >= 0);
        let length = length as usize;
        assert!(length <= N);
        let string =
            std::str::from_utf8(input.input.slice((input.offset)..(input.offset + length)))
                .expect("invalid utf8. that's a skill issue ngl");
        input.offset += length;

        // SAFETY: checked length being <= N, can unwrap_unchecked here.
        Ok((input, unsafe {
            FixedStr::from_string(string).unwrap_unchecked()
        }))
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
    type RefC<T: ?Sized>: Clone + Deref<Target = T>
    where
        Self::RefC<str>: Display;

    fn refc_from_str(s: &str) -> Self::RefC<str>;
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]>;
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]>;
}

#[derive(Clone, Copy, Debug)]
pub struct YesSync;
#[derive(Clone, Copy, Debug)]
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
    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        let (input, VarInt(length)) = VarInt::<i32>::deserialize(input)?;
        assert!(length >= 0);
        let length = length as usize;
        let slic = input.input.slice(input.offset..input.offset + length);
        match serde_json::from_slice::<T>(slic) {
            Ok(j) => Ok((input, Self(j))),
            Err(e) => Err((input, crate::error::Error::Json(e))),
        }
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    type Context = T::Context;

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        let (input, exists) = one_of([0x0, 0x1]).map(|t| t == 0x1).parse(input)?;
        if exists {
            let (input, value) = T::deserialize(input)?;
            Ok((input, Some(value)))
        } else {
            Ok((input, None))
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
    fn deserialize<'parse, 'a>(
        mut input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        const AMOUNT: usize = 16;

        if input.input.len() < input.offset + AMOUNT {
            let e = Error::unexpected_eof(input.span_since(input.offset), None);
            return Err((input, e));
        }

        let bytes = input.input.slice(input.offset..input.offset + AMOUNT);
        input.offset += AMOUNT;

        Ok((input, Self::from_slice(bytes).unwrap()))
    }
}

#[derive(Debug, Deref)]
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
    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        let (input, VarInt::<i32>(length)) = deser_cx(input)?;
        assert!(length >= 0);
        let length = length as usize;

        T::deserialize
            .repeated_custom::<Self>()
            .exactly(length)
            .parse(input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[display(fmt = "Identifier({_0}:{_1})")]
pub struct Identifier<Sy: Syncable = YesSync>(pub Namespace, pub Sy::RefC<str>);
impl<Sy: Syncable> Identifier<Sy> {
    pub fn new(namespace: Namespace, value: &str) -> Self {
        Self(namespace, Sy::refc_from_str(value))
    }

    fn parse<'parse, 'a>(
        input: aott::input::Input<'parse, &'a str, Extra<()>>,
    ) -> aott::error::IResult<'parse, &'a str, Extra<()>, Self> {
        tuple((
            aott::text::ascii::ident.map(|namespace| match namespace {
                "minecraft" => Namespace::Minecraft,
                anything_else => Namespace::Custom(Cow::Owned(anything_else.to_owned())),
            }),
            just(":").ignored(),
            aott::text::ascii::ident.map(Sy::refc_from_str),
        ))
        .map(|(namespace, (), value)| Self(namespace, value))
        .parse(input)
    }
}

impl<Sy: Syncable> Serialize for Identifier<Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let formatted = format!("{}:{}", self.0, self.1);
        formatted.as_str().serialize_to(buf)
    }
}
impl<Sy: Syncable> Deserialize for Identifier<Sy> {
    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        let (input, FixedStr::<32767, Sy> { inner }) = FixedStr::deserialize(input)?;
        match Self::parse.parse_from(&&inner).into_result() {
            Ok(ok) => Ok((input, ok)),
            Err(err) => Err((input, err)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Namespace {
    #[display(fmt = "minecraft")]
    Minecraft,
    #[display(fmt = "{_0}")]
    Custom(Cow<'static, str>),
}
