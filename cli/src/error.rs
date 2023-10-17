use std::{borrow::Cow, num::ParseIntError, ops::Range};

use oxcr_protocol::{
    aott::{
        self,
        error::LabelError,
        text::{self, CharError, CharLabel},
    },
    miette::{self, SourceSpan},
    ser::any_of,
    thiserror,
    tracing::metadata::ParseLevelError,
};

#[derive(miette::Diagnostic, thiserror::Error, Debug)]
pub enum ParseError {
    #[error("expected {expected}, found {found}")]
    #[diagnostic(code(aott::error::expected))]
    Expected {
        expected: Expectation,
        found: char,
        #[label = "here"]
        at: SourceSpan,
        #[help]
        help: Option<Cow<'static, str>>,
    },
    #[error("unexpected end of input{}", .expected.as_ref().map(|expectation| format!(", expected {expectation}")).unwrap_or_else(String::new))]
    #[diagnostic(
        code(aott::error::unexpected_eof),
        help("try giving it more input next time, I guess?")
    )]
    UnexpectedEof {
        #[label = "here"]
        at: SourceSpan,
        #[label = "last data here"]
        last_data_at: Option<SourceSpan>,
        expected: Option<Expectation>,
    },
    #[error("parsing a Level failed: {actual}")]
    #[diagnostic(code(cli::parse_level_error))]
    ParseLevel {
        actual: ParseLevelError,
        #[label = "here"]
        at: SourceSpan,
    },
    #[error("unknown flag encountered: {flag}")]
    #[diagnostic(code(cli::unknown_flag))]
    UnknownFlag {
        flag: String,
        #[label = "here"]
        at: SourceSpan,
    },
    #[error("expected a number with radix {radix}, got {actual}")]
    #[diagnostic(code(cli::expected_number))]
    ExpectedNumber {
        radix: u32,
        actual: String,
        #[label = "here"]
        at: SourceSpan,
        #[help]
        help: Option<Cow<'static, str>>,
        #[source]
        error: ParseIntError,
    },

    #[error("{label}; last token was {last_token:?}")]
    Text {
        #[label = "here"]
        at: SourceSpan,
        label: aott::text::CharLabel<char>,
        last_token: Option<char>,
    },

    #[error("{label}; last token was {last_token:?}")]
    Builtin {
        #[label = "here"]
        at: SourceSpan,
        label: aott::error::BuiltinLabel,
        last_token: Option<char>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Expectation {
    #[error("{}", any_of(.0))]
    AnyOf(Vec<char>),
    #[error("{}", any_of(.0))]
    AnyOfStr(Vec<&'static str>),
    #[error("end of input")]
    EndOfInput,
    #[error("a digit with radix {_0}")]
    Digit(u32),
}

impl<'a> aott::error::FundamentalError<&'a str> for ParseError {
    fn unexpected_eof(
        span: Range<usize>,
        expected: Option<Vec<<&'a str as aott::prelude::InputType>::Token>>,
    ) -> Self {
        Self::UnexpectedEof {
            at: span.into(),
            last_data_at: None,
            expected: expected.map(Expectation::AnyOf),
        }
    }

    fn expected_token_found(span: Range<usize>, expected: Vec<char>, found: char) -> Self {
        Self::Expected {
            expected: Expectation::AnyOf(expected),
            found,
            at: span.into(),
            help: None,
        }
    }

    fn expected_eof_found(span: Range<usize>, found: char) -> Self {
        Self::Expected {
            expected: Expectation::EndOfInput,
            found,
            at: span.into(),
            help: None,
        }
    }
}

impl<'a> LabelError<&'a str, CharLabel<char>> for ParseError {}
