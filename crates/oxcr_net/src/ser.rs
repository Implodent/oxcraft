use std::{marker::PhantomData, ops::Range};

use ::bytes::{BufMut, BytesMut};
use aott::prelude::{bytes as b, *};
use fstr::FStr;

use crate::model::{LEB128Number, VarInt};

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

impl<const N: usize> Deserialize for fstr::FStr<N> {
    type Context = ();

    fn deserialize<'parse, 'a>(
        input: Inp<'parse, 'a, Self::Context>,
    ) -> Resul<'parse, 'a, Self, Self::Context> {
        // get length, must be 0 <= length <= N
        let (input, VarInt(length)) = VarInt::<i32>::deserialize(input)?;
        assert!(length >= 0);
        let length = length as usize;
        assert!(length <= N);
        let mut string = [0u8; N];
        string.clone_from_slice(input.input.slice(input.offset..input.offset + length));
        match fstr::FStr::from_inner(string) {
            Ok(fstr) => Ok((input, fstr)),
            Err(e) => Err((input, e.into())),
        }
    }
}

impl<const N: usize> Serialize for FStr<N> {
    fn serialize_to(&self, buf: &mut BytesMut) {
        let n: i32 = N.try_into().unwrap();
        let ln = VarInt(n);
        buf.reserve(ln.length_of() + N);
        ln.serialize_to(buf);
        buf.put_slice(&self.as_bytes()[..])
    }
}
