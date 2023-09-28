use std::ops::Range;

use oxcr_protocol::{
    aott,
    miette::{self, SourceSpan},
    ser::any_of,
    thiserror,
};

#[derive(miette::Diagnostic, thiserror::Error, Debug)]
pub enum ParseErrorKind {
    #[error("expected {expected}, found {found}")]
    Expected { expected: Expectation, found: char },
    #[error("unexpected end of input")]
    UnexpectedEof,
}

#[derive(Debug, thiserror::Error)]
pub enum Expectation {
    #[error("{}", any_of(.0))]
    AnyOf(Vec<char>),
    #[error("end of input")]
    EndOfInput,
    #[error("a digit with radix {_0}")]
    Digit(u32),
}

#[derive(miette::Diagnostic, thiserror::Error, Debug)]
#[error("{kind}")]
pub struct ParseError {
    #[label = "here"]
    pub span: SourceSpan,
    #[diagnostic(transparent)]
    #[diagnostic_source]
    #[source]
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub fn new(span: Range<usize>, kind: ParseErrorKind) -> Self {
        Self {
            span: span.into(),
            kind,
        }
    }
}

impl<'a> aott::error::Error<&'a str> for ParseError {
    type Span = Range<usize>;

    fn unexpected_eof(
        span: Self::Span,
        _expected: Option<Vec<<&'a str as aott::prelude::InputType>::Token>>,
    ) -> Self {
        Self::new(span, ParseErrorKind::UnexpectedEof)
    }

    fn expected_token_found(
        span: Self::Span,
        expected: Vec<char>,
        found: aott::MaybeRef<'_, char>,
    ) -> Self {
        Self::new(
            span,
            ParseErrorKind::Expected {
                expected: Expectation::AnyOf(expected),
                found: found.into_clone(),
            },
        )
    }

    fn expected_eof_found(span: Self::Span, found: aott::MaybeRef<'_, char>) -> Self {
        Self::new(
            span,
            ParseErrorKind::Expected {
                expected: Expectation::EndOfInput,
                found: found.into_clone(),
            },
        )
    }
}
