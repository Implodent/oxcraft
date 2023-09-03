use derive_more::*;
use std::{
    marker::PhantomData,
    ops::{Deref, Range},
    rc::Rc,
    sync::Arc,
};

use crate::model::{LEB128Number, VarInt};
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
        let (input, VarInt(length)) = VarInt::<i32>::deserialize(input)?;
        debug!(%length, expected_length=%N, "[fixedstr] checking length");
        assert!(length >= 0);
        let length = length as usize;
        assert!(length <= N);
        let string =
            std::str::from_utf8(input.input.slice((input.offset)..(input.offset + length)))
                .expect("invalid utf8. that's a skill issue ngl");
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
        self.inner.fmt(f)
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
    type RefC<T: ?Sized>: Clone + Deref<Target = T>;

    fn refc_from_str(s: &str) -> Self::RefC<str>;
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
}
impl Syncable for NoSync {
    const SYNC: bool = false;
    type RefC<T: ?Sized> = Rc<T>;
    fn refc_from_str(s: &str) -> Self::RefC<str> {
        Rc::from(s)
    }
}

impl<'a> Serialize for &'a str {
    fn serialize_to(&self, buf: &mut BytesMut) {
        VarInt::<i32>(self.len().try_into().unwrap()).serialize_to(buf);
        buf.put_slice(self.as_bytes());
    }
}

#[derive(Clone, Copy, Deref, DerefMut, Debug, Display)]
pub struct Json<T>(pub T);

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
