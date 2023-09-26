//! Spanned-clap, lol

use std::{marker::PhantomData, path::PathBuf};

use aott::prelude::*;
use oxcr_protocol::{
    aott::{self, pfn_type},
    bytes::Bytes,
};

use crate::error::{ParseError, ParseErrorKind};

pub struct Extra<C = ()>(PhantomData<C>);
impl<'a, C> ParserExtras<&'a str> for Extra<C> {
    type Context = C;
    type Error = crate::error::ParseError;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliSerOp {
    Encode,
    Decode,
}

#[derive(Debug, Clone)]
pub enum ByteInput {
    File(PathBuf),
    Data(Bytes),
}

#[derive(Debug, Clone)]
pub struct CliSer {
    pub operation: CliSerOp,
    pub inp: ByteInput,
}

#[derive(Debug, Clone)]
pub enum Cli {
    Serialization(CliSer),
}

fn radixshit<'a>(radix: u32, cha: char) -> pfn_type!(&'a str, Bytes, Extra) {
    |input| {
        just(['0', cha])
            .ignore_then(text::int(radix))
            .try_map_with_span(|x: &str, span| {
                x.chars()
                    .map(|c| {
                        c.to_digit(radix)
                            .ok_or_else(|| ParseError {
                                span: span.into(),
                                kind: ParseErrorKind::Expected {
                                    expected: Expectation::Digit(8),
                                    found: c,
                                },
                            })
                            .map(|x| x.try_into().expect("not a u8"))
                    })
                    .try_collect()
            })
            .parse_with(input)
    }
}

#[parser(extras = Extra)]
fn byte_input(input: &str) -> ByteInput {
    choice((radixshit(8, 'o'),))
        .map(ByteInput::Data)
        .parse_with(input)
}

#[parser(extras = Extra)]
pub fn yay(input: &str) -> Cli {}
