use ::bytes::Bytes;
use aott::prelude::*;

pub trait Deserialize {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, Self>;
}

pub trait Serialize {
    fn serialize(&self) -> Bytes;
}

pub type Inp<'parse, 'a> = Input<'parse, &'a [u8], extra::Err<&'a [u8]>>;
pub type Res<'parse, 'a, T> = IResult<'parse, &'a [u8], extra::Err<&'a [u8]>, T>;

pub fn deser<'parse, 'a, T: Deserialize>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, T> {
    T::deserialize(input)
}

pub fn seri<T: Serialize>(t: &T) -> Bytes {
    t.serialize()
}

#[parser(extras = E)]
pub fn slice_till_end<'a, I: SliceInput<'a>, E: ParserExtras<I>>(input: I) -> I::Slice {
    let slice = input.input.slice_from(input.offset);
    Ok((input, slice))
}
