use std::{borrow::Cow, num::ParseIntError, ops::Range};

use oxcr_protocol::{
    aott::{
        self,
        text::{self, CharError},
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
    #[error("filter failed in {location}, while checking {token}")]
    #[diagnostic(code(aott::error::filter_failed))]
    FilterFailed {
        #[label = "this is the token that didn't pass"]
        at: SourceSpan,
        location: &'static core::panic::Location<'static>,
        token: char,
    },
    #[error("expected keyword {keyword}")]
    #[diagnostic(code(aott::text::error::expected_keyword))]
    ExpectedKeyword {
        #[label = "the keyword here is {found}"]
        at: SourceSpan,
        keyword: String,
        found: String,
    },
    #[error("expected digit of radix {radix}")]
    #[diagnostic(code(aott::text::error::expected_digit))]
    ExpectedDigit {
        #[label = "here"]
        at: SourceSpan,
        radix: u32,
        found: char,
    },
    #[error("expected identifier character (a-zA-Z or _), but found {found}")]
    ExpectedIdent {
        #[label = "here"]
        at: SourceSpan,
        found: char,
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

impl<'a> aott::error::Error<&'a str> for ParseError {
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

    fn filter_failed(
        span: Range<usize>,
        location: &'static core::panic::Location<'static>,
        token: <&'a str as aott::prelude::InputType>::Token,
    ) -> Self {
        Self::FilterFailed {
            at: span.into(),
            location,
            token,
        }
    }
}

impl CharError<char> for ParseError {
    fn expected_digit(span: Range<usize>, radix: u32, got: char) -> Self {
        Self::ExpectedDigit {
            at: span.into(),
            radix,
            found: got,
        }
    }

    fn expected_ident_char(span: Range<usize>, got: char) -> Self {
        Self::ExpectedIdent {
            at: span.into(),
            found: got,
        }
    }

    fn expected_keyword<'a, 'b: 'a>(
        span: Range<usize>,
        keyword: &'b <char as text::Char>::Str,
        actual: &'a <char as text::Char>::Str,
    ) -> Self {
        Self::ExpectedKeyword {
            at: span.into(),
            keyword: keyword.to_owned(),
            found: actual.to_owned(),
        }
    }
}
