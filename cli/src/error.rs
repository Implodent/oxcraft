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
}

#[derive(miette::Diagnostic, thiserror::Error, Debug)]
pub struct ParseError {
    #[label = "here"]
    pub span: SourceSpan,
}
