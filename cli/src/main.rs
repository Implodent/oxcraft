#![allow(dead_code)]
#![feature(iterator_try_collect, try_blocks)]

use oxcr_protocol::{
    aott::{self, prelude::Parser},
    bytes::Bytes,
    error::Error,
    logging::CraftLayer,
    miette::{bail, IntoDiagnostic, Report},
    model::{
        packets::{play::LoginPlay, Packet, SerializedPacket},
        VarInt,
    },
    nbt::Nbt,
    ser::{BytesSource, Deserialize, WithSource},
    tracing::debug,
};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::cli::{ByteInput, CliCommand};

mod cli;
mod error;

fn run(_path: String, args: &str) -> Result<(), Report> {
    let tracing_set_default_level = tracing_subscriber::registry()
        .with(EnvFilter::from_env("OXCR_LOG"))
        .with(CraftLayer)
        .set_default();

    let cli = cli::yay
        .parse(args)
        .map_err(|parse_error| Report::new(parse_error).with_source_code(args.to_owned()))?;

    drop(tracing_set_default_level);

    let _tracing_set_real_level = tracing_subscriber::registry()
        .with(EnvFilter::new(cli.level.to_string()))
        .with(CraftLayer)
        .set_default();

    debug!(%cli.level, ?cli.command);

    match cli.command {
        CliCommand::Help => help(),
        CliCommand::Decode(inp) => {
            let bytes = read_byte_input(inp)?;
            debug!("{bytes:?}");
            let spack = SerializedPacket::deserialize.parse(&bytes)?;
            let deserialized: LoginPlay = spack.try_deserialize(LoginPlay::STATE)?;

            println!("{:#?}", deserialized);
        }
        CliCommand::VarInt(inp) => {
            let bytes = read_byte_input(inp)?;

            println!(
                "{:#?}",
                VarInt::<i64>::deserialize
                    .then_ignore(aott::prelude::end)
                    .parse(&bytes)
                    .map_err(reconcile(bytes))?
            );
        }
        CliCommand::Nbt(tag, inp) => {
            let bytes = read_byte_input(inp)?;
            let nbt = Nbt::single
                .parse_with_context(&bytes, tag)
                .map_err(reconcile(bytes))?;

            println!("{nbt:#?}");
        }
        CliCommand::Decompress(_inp) => todo!(),
    };

    Ok(())
}

fn reconcile(
    bytes: Bytes,
) -> impl FnOnce(oxcr_protocol::error::Error) -> oxcr_protocol::error::Error {
    move |error| match error {
        Error::Ser(ser) => Error::SerSrc(WithSource {
            kind: ser.kind,
            span: ser.span,
            source: BytesSource::new(bytes, Some("thing.bin".into())),
        }),
        a => a,
    }
}

fn read_byte_input(inp: ByteInput) -> Result<Bytes, Report> {
    try {
        match inp {
            ByteInput::Data(data) => data,
            ByteInput::File(file) => {
                std::fs::read(std::env::current_dir().into_diagnostic()?.join(file))
                    .into_diagnostic()?
                    .into()
            }
        }
    }
}

fn main() -> Result<(), Report> {
    let mut args = std::env::args();

    match args.next() {
        None => bail!("the"),
        Some(path) => {
            let args = args.collect::<Vec<String>>().join(" ");
            run(path, &args)?;

            Ok(())
        }
    }
}

fn help() {
    println!(
        r#"
This is the OxCraft CLI. Here you can serialize and deserialize packets (currently only LoginPlay) and NBT.

Example usage:
cargo run -p oxcr_cli -- -Dd 0xbd7d9a9f7e
This will turn on debug logging and deserialize LoginPlay from the data 0xbd7d etc.

Clarifications:
<DATA> is either inline binary (0b...), octal (0o...), hexadecimal (0x...), or decimal data (0d...); or it could be a path like ./mydata.bin

Flags:
-l --level <LEVEL> what log level (error, warn, info, debug, trace)
-D --debug same as --level debug
-d --deserialize --decode <DATA> deserializes a packet from <DATA>, then debug-logs it.
-n --nbt <TAG_TYPE>, <DATA> deserializes a <TAG_TYPE> from <DATA>. <TAG_TYPE> is (right now only) compound
    "#
    );
}
