//! Spanned-clap, lol

use std::{marker::PhantomData, path::PathBuf, str::FromStr};

use aott::prelude::*;
use oxcr_protocol::{
    aott::{
        self, pfn_type,
        text::{ascii::ident, inline_whitespace, int, whitespace},
    },
    bytes::{BufMut, Bytes, BytesMut},
    tracing::{level_filters::LevelFilter, Level},
};

use crate::error::{Expectation, ParseError, ParseErrorKind};

pub struct Extra<C = ()>(PhantomData<C>);
impl<'a, C> ParserExtras<&'a str> for Extra<C> {
    type Context = C;
    type Error = crate::error::ParseError;
}

#[derive(Debug, Clone)]
pub enum ByteInput {
    File(PathBuf),
    Data(Bytes),
}

#[derive(Debug, Clone)]
pub enum CliCommand {
    Decode(ByteInput),
    Help,
}

#[derive(Debug, Clone)]
pub struct Cli {
    pub level: LevelFilter,
    pub command: CliCommand,
}

#[derive(Debug, Clone)]
pub enum Flag {
    Level(Level),
    Help,
    Decode(ByteInput),
}

#[derive(Debug)]
enum FlagName<'a> {
    Short(&'a str),
    Long(&'a str),
}

fn numbah<'a>(radix: u32, cha: char) -> pfn_type!(&'a str, Bytes, Extra) {
    move |input| {
        just(['0', cha])
            .ignore_then(text::int(radix))
            .try_map_with_span(|x: &str, span| {
                x.chars()
                    .map(|c| {
                        c.to_digit(radix)
                            .ok_or_else(|| ParseError {
                                span: span.clone().into(),
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
    choice((
        numbah(16, 'x'),
        numbah(2, 'b'),
        numbah(8, 'o'),
        numbah(10, 'd'),
    ))
    .or(text::int(16).try_map_with_span(|x: &str, span| {
        x.chars()
            .map(|c| {
                c.to_digit(16)
                    .ok_or_else(|| ParseError {
                        span: span.clone().into(),
                        kind: ParseErrorKind::Expected {
                            expected: Expectation::Digit(8),
                            found: c,
                        },
                    })
                    .map(|x| x.try_into().expect("not a u8"))
            })
            .try_collect()
    }))
    .map(ByteInput::Data)
    .parse_with(input)
}

#[parser(extras = Extra)]
fn flag_list(input: &str) -> Vec<FlagName<'a>> {
    if input.offset.saturating_sub(1) >= input.input.len() {
        return Ok(vec![]);
    }
    just("-")
        .ignore_then(choice((
            slice(filter(|thing: &char| *thing != ' '))
                .map(FlagName::Short)
                .repeated(),
            just("-")
                .ignore_then(ident)
                .map(|name| vec![FlagName::Long(name)]),
        )))
        .parse_with(input)
}

#[parser(extras = Extra)]
fn flags(input: &str) -> Vec<Flag> {
    let before = input.offset;

    flag_list(input)?
        .into_iter()
        .map(|flag| -> Result<Flag, crate::error::ParseError> {
            try {
                match flag {
                    FlagName::Short("l") | FlagName::Long("level") => {
                        one_of(" =")(input)?;
                        let before_ = input.offset;
                        Flag::Level(
                            Level::from_str(ident.or(slice(int(10))).parse_with(input)?).map_err(
                                |error| ParseError {
                                    span: (before_, input.offset).into(),
                                    kind: error.into(),
                                },
                            )?,
                        )
                    }
                    FlagName::Short("D") | FlagName::Long("debug") => Flag::Level(Level::DEBUG),
                    FlagName::Short("d")
                    | FlagName::Long("decode")
                    | FlagName::Long("deserialize") => {
                        one_of(" =")(input)?;

                        Flag::Decode(byte_input(input)?)
                    }
                    FlagName::Short("h") | FlagName::Long("help") => Flag::Help,
                    FlagName::Short(flag) | FlagName::Long(flag) => Err(ParseError {
                        span: input.span_since(before).into(),
                        kind: ParseErrorKind::UnknownFlag(flag.to_owned()),
                    })?,
                }
            }
        })
        .try_collect()
}

fn flags_handle<'a>(
    cli: &mut Cli,
    input: &mut Input<&'a str, Extra>,
) -> PResult<&'a str, (), Extra> {
    fn handle(cli: &mut Cli, flag: Flag) {
        match flag {
            Flag::Level(level) => cli.level = LevelFilter::from(level),
            Flag::Help => cli.command = CliCommand::Help,
            Flag::Decode(input) => cli.command = CliCommand::Decode(input),
        }
    }

    try { flags(input)?.into_iter().for_each(|flag| handle(cli, flag)) }
}

#[parser(extras = Extra)]
pub fn yay(input: &str) -> Cli {
    try {
        let mut cli = Cli {
            level: LevelFilter::OFF,
            command: CliCommand::Help,
        };

        flags_handle(&mut cli, input)?;

        cli
    }
}
