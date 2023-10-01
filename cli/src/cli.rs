//! Spanned-clap, lol

use itertools::Itertools;
use std::{borrow::Cow, marker::PhantomData, path::PathBuf, str::FromStr};

use aott::prelude::*;
use oxcr_protocol::{
    aott::{
        self, pfn_type,
        text::{ascii::ident, digits, inline_whitespace},
    },
    bytes::Bytes,
    nbt::NbtTagType,
    tracing::{level_filters::LevelFilter, Level},
};

use crate::error::{Expectation, ParseError};

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
    VarInt(ByteInput),
    Decompress(ByteInput),
    Nbt(NbtTagType, ByteInput),
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
    VarInt(ByteInput),
    Decompress(ByteInput),
    Nbt(NbtTagType, ByteInput),
}

#[derive(Debug)]
pub enum FlagName<'a> {
    Short(&'a str),
    Long(&'a str),
}

fn numbah<'a>(radix: u32, cha: char) -> pfn_type!(&'a str, Bytes, Extra) {
    move |input| {
        just(['0', cha])
            .ignore_then(text::digits(radix).slice())
            .map(|x: &str| {
                x.chars()
                    .batching(|it| {
                        let mut s = it.next()?.to_string();
                        it.next().map(|c| s.push(c));
                        Some(u8::from_str_radix(&s, radix).expect("a u8..."))
                    })
                    .collect()
            })
            .parse_with(input)
    }
}

#[parser(extras = Extra)]
fn path_buf(input: &str) -> PathBuf {
    Ok(PathBuf::from_str(input.input.slice_from(input.offset..)).unwrap())
}

#[parser(extras = Extra)]
fn byte_input(input: &str) -> ByteInput {
    (choice((
        numbah(16, 'x'),
        numbah(2, 'b'),
        numbah(8, 'o'),
        numbah(10, 'd'),
    ))
    .map(ByteInput::Data))
    .or(path_buf.map(ByteInput::File))
    .parse_with(input)
}

#[parser(extras = Extra)]
pub fn flag_list(input: &str) -> Vec<FlagName<'a>> {
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
                            Level::from_str(ident.or(slice(digits(10))).parse_with(input)?)
                                .map_err(|actual| ParseError::ParseLevel {
                                    at: (before_, input.offset).into(),
                                    actual,
                                })?,
                        )
                    }
                    FlagName::Short("D") | FlagName::Long("debug") => Flag::Level(Level::DEBUG),
                    FlagName::Short("d")
                    | FlagName::Long("decode")
                    | FlagName::Long("deserialize") => {
                        one_of(" =")(input)?;

                        Flag::Decode(byte_input(input)?)
                    }
                    FlagName::Short("c") | FlagName::Long("decompress") => {
                        one_of(" =")(input)?;

                        Flag::Decompress(byte_input(input)?)
                    }
                    FlagName::Short("V") | FlagName::Long("varint") => {
                        one_of(" =")(input)?;

                        Flag::VarInt(byte_input(input)?)
                    }
                    FlagName::Short("h") | FlagName::Long("help") => Flag::Help,
                    FlagName::Short("n") | FlagName::Long("nbt") => {
                        one_of(" =")(input)?;
                        let before_nbt_tag = input.offset;

                        let nbt_tag = match ident(input)? {
                            "compound" => NbtTagType::Compound,
                            tag => {
                                return Err(ParseError::Expected {
                                    expected: Expectation::AnyOfStr(vec!["compound"]),
                                    found: tag.chars().next().expect("no tag at all"),
                                    at: input.span_since(before_nbt_tag).into(),
                                    help: Some(Cow::Borrowed("an NBT tag type is: Compound")),
                                })
                            }
                        };

                        just(",")(input)?;

                        inline_whitespace().check_with(input)?;

                        Flag::Nbt(nbt_tag, byte_input(input)?)
                    }
                    FlagName::Short(flag) | FlagName::Long(flag) => Err(ParseError::UnknownFlag {
                        flag: flag.to_owned(),
                        at: input.span_since(before).into(),
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
            Flag::VarInt(input) => cli.command = CliCommand::VarInt(input),
            Flag::Decompress(input) => cli.command = CliCommand::Decompress(input),
            Flag::Nbt(tag, input) => cli.command = CliCommand::Nbt(tag, input),
        }
    }
    try { flags(input)?.into_iter().for_each(|flag| handle(cli, flag)) }
}

#[parser(extras = Extra)]
pub fn yay(input: &str) -> Cli {
    try {
        let mut cli = Cli {
            level: LevelFilter::INFO,
            command: CliCommand::Help,
        };

        flags_handle(&mut cli, input)?;

        cli
    }
}
