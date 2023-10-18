use std::{borrow::Cow, num::ParseIntError, ops::Range};

use oxcr_protocol::{
    aott::{
        self,
        error::{BuiltinLabel, LabelError},
        primitive::SeqLabel,
        text::CharLabel,
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
    #[diagnostic(code(aott::error::unexpected_eof))]
    UnexpectedEof {
        #[label = "here"]
        at: SourceSpan,
        #[label = "last data here"]
        last_data_at: Option<SourceSpan>,
        expected: Option<Expectation>,
        #[help]
        help: Option<Cow<'static, str>>,
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

    #[error("expected {label:?}; last token was {last_token:?}")]
    Sequence {
        #[label = "here"]
        at: SourceSpan,
        label: SeqLabel<char>,
        last_token: Option<char>,
    },
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum Expectation {
    #[error("{}", any_of(.0))]
    AnyOf(Vec<char>),
    #[error("{}", any_of(.0))]
    AnyOfStr(Vec<&'static str>),
    #[error("end of input")]
    EndOfInput,
    #[error("a digit with radix {_0}")]
    Digit(u32),
    #[error("a short flag character (anything but a whitespace)")]
    ShortFlag,
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
            help: None,
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

impl<'a> LabelError<&'a str, CharLabel<char>> for ParseError {
    fn from_label(
        span: Range<usize>,
        label: CharLabel<char>,
        last_token: Option<<&'a str as aott::prelude::InputType>::Token>,
    ) -> Self {
        Self::Text {
            at: span.into(),
            label,
            last_token,
        }
    }
}

impl<'a> LabelError<&'a str, BuiltinLabel> for ParseError {
    fn from_label(
        span: Range<usize>,
        label: BuiltinLabel,
        last_token: Option<<&'a str as aott::prelude::InputType>::Token>,
    ) -> Self {
        Self::Builtin {
            at: span.into(),
            label,
            last_token,
        }
    }
}

impl<'a> LabelError<&'a str, SeqLabel<char>> for ParseError {
    fn from_label(
        span: Range<usize>,
        label: SeqLabel<char>,
        last_token: Option<<&'a str as aott::prelude::InputType>::Token>,
    ) -> Self {
        Self::Sequence {
            at: span.into(),
            label,
            last_token,
        }
    }
}

impl<'a> LabelError<&'a str, Expectation> for ParseError {
    fn from_label(
        span: Range<usize>,
        label: Expectation,
        last_token: Option<<&'a str as aott::prelude::InputType>::Token>,
    ) -> Self {
        if let Some(found) = last_token {
            Self::Expected {
                at: span.into(),
                expected: label,
                found,
                help: None,
            }
        } else {
            let last_data_at = (span.start.saturating_sub(1), span.end.saturating_sub(1)).into();
            Self::UnexpectedEof {
                at: span.into(),
                last_data_at: Some(last_data_at),
                expected: Some(label),
                help: None,
            }
        }
    }
}

impl<'a> LabelError<&'a str, (Expectation, Cow<'static, str>)> for ParseError {
    fn from_label(
        span: Range<usize>,
        (label, help): (Expectation, Cow<'static, str>),
        last_token: Option<<&'a str as aott::prelude::InputType>::Token>,
    ) -> Self {
        if let Some(found) = last_token {
            Self::Expected {
                at: span.into(),
                expected: label,
                found,
                help: Some(help),
            }
        } else {
            let last_data_at = (span.start.saturating_sub(1), span.end.saturating_sub(1)).into();
            Self::UnexpectedEof {
                at: span.into(),
                last_data_at: Some(last_data_at),
                expected: Some(label),
                help: Some(help),
            }
        }
    }
}
