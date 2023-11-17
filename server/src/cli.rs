use oxcr_cli::{flag_list, Extra, FlagName, ParseError};
use oxcr_protocol::{
    aott::{
        self,
        prelude::{Parser, *},
    },
    miette::{self, bail, IntoDiagnostic},
};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Cli {
    pub port: u16,
}

impl Cli {
    pub fn parse_args() -> miette::Result<Self> {
        let mut args = std::env::args();

        match args.next() {
            None => bail!("the"),
            Some(_path) => {
                let args = args.collect::<Vec<String>>().join(" ");

                yay.parse(&args).into_diagnostic()
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Flag {
    Port(u16),
}

#[parser(extras = Extra)]
fn flags(input: &str) -> Vec<Flag> {
    let before = input.offset;

    flag_list(input)?
        .into_iter()
        .map(|flag| -> Result<Flag, ParseError> {
            try {
                match flag {
                    FlagName::Short("p") | FlagName::Long("port") => {
                        one_of(" =")(input)?;

                        Flag::Port(text::int(10).try_map(|int: &str, extra| {
                            int.parse::<u16>().map_err(|error| ParseError::ExpectedNumber {
                                radix: 10,
                                actual: int.to_owned(),
                                at: Into::<miette::SourceSpan>::into(extra.span()),
                                help: Some(Cow::Borrowed("a port can only be a u16 (0 ... 65535), but your input either didn't fit into a u16 or isn't a number at all.")),
                                error
                            })
                        }).parse_with(input)?)
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
            Flag::Port(port) => cli.port = port,
        }
    }
    try { flags(input)?.into_iter().for_each(|flag| handle(cli, flag)) }
}

#[parser(extras = Extra)]
pub fn yay(input: &str) -> Cli {
    try {
        let mut cli = Cli { port: 25565 };

        flags_handle(&mut cli, input)?;

        cli
    }
}
