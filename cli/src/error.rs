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
