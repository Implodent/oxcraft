use aott::text::{Char, CharLabel};

use super::*;

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum SerializationError<Item: Debug + Display + Char> {
    #[error("expected {}, found {found}", any_of(.expected))]
    #[diagnostic(code(aott::error::expected), help("invalid inputs encountered - try looking at it more and check if you got something wrong."))]
    Expected {
        expected: Vec<Item>,
        found: Item,
        #[label = "here"]
        at: SourceSpan,
    },

    #[error("unexpected end of file")]
    #[diagnostic(
        code(aott::error::unexpected_eof),
        help("there wasn't enough input to deserialize, try giving more next time.")
    )]
    UnexpectedEof {
        expected: Option<Vec<Item>>,
        #[label = "last input was here"]
        at: SourceSpan,
    },

    #[error("expected end of input, found {found}")]
    #[diagnostic(
        code(aott::error::expected_eof),
        help("more input was given than expected, try revising your inputs.")
    )]
    ExpectedEof {
        found: Item,
        #[label = "end of input was expected here"]
        at: SourceSpan,
    },

    #[error("{label}; last token is {last_token:?}")]
    Type {
        #[label = "here"]
        at: SourceSpan,
        #[diagnostic_source]
        #[source]
        #[diagnostic(transparent)]
        label: super::types::Label,
        last_token: Option<Item>,
    },

    #[error("text parsing error: {label}")]
    String {
        #[label = "here"]
        at: SourceSpan,
        label: aott::text::CharLabel<Item>,
        last_token: Option<Item>,
    },

    #[error("NBT error: {label}")]
    Nbt {
        #[label = "here"]
        at: SourceSpan,
        #[diagnostic_source]
        #[source]
        #[diagnostic(transparent)]
        label: crate::nbt::Label,
        last_token: Option<Item>,
    },
}

macro_rules! label_error {
    ($variant:ident; u8 => $u8:ty) => {
        impl<'a> aott::error::LabelError<&'a [u8], $u8> for crate::error::Error {
            fn from_label(span: Range<usize>, label: $u8, last_token: Option<u8>) -> Self {
                Self::Ser(SerializationError::$variant {
                    at: span.into(),
                    label,
                    last_token,
                })
            }
        }
    };
    ($variant:ident; char => $char:ty) => {
        impl<'a> aott::error::LabelError<&'a str, $char> for crate::error::Error {
            fn from_label(span: Range<usize>, label: $char, last_token: Option<char>) -> Self {
                Self::SerStr(SerializationError::$variant {
                    at: span.into(),
                    label,
                    last_token,
                })
            }
        }
    };
    ($variant:ident => $u8:ty; $char:ty) => {
        label_error!($variant; u8 => $u8);
        label_error!($variant; char => $char);
    };
    ($laty:ty => $variant:ident) => {
        label_error!($variant; u8 => $laty);
        label_error!($variant; char => $laty);
    }
}

label_error!(super::types::Label => Type);
label_error!(String => CharLabel<u8>; CharLabel<char>);
label_error!(Nbt; u8 => crate::nbt::Label);

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("{errors:#?}")]
pub struct WithSource<Item: Debug + Display + Char + 'static> {
    #[source_code]
    pub src: BytesSource,
    #[related]
    pub errors: Vec<SerializationError<Item>>,
}

#[derive(Debug, Clone)]
pub struct BytesSource(Bytes, Option<String>);

fn context_info<'a>(
    input: &'a [u8],
    span: &SourceSpan,
    context_lines_before: usize,
    context_lines_after: usize,
    name: Option<String>,
) -> Result<miette::MietteSpanContents<'a>, miette::MietteError> {
    let mut offset = 0usize;
    let mut line_count = 0usize;
    let mut start_line = 0usize;
    let mut start_column = 0usize;
    let mut before_lines_starts = VecDeque::new();
    let mut current_line_start = 0usize;
    let mut end_lines = 0usize;
    let mut post_span = false;
    let mut post_span_got_newline = false;
    let mut iter = input.iter().copied().peekable();
    while let Some(char) = iter.next() {
        if matches!(char, b'\r' | b'\n') {
            line_count += 1;
            if char == b'\r' && iter.next_if_eq(&b'\n').is_some() {
                offset += 1;
            }
            if offset < span.offset() {
                // We're before the start of the span.
                start_column = 0;
                before_lines_starts.push_back(current_line_start);
                if before_lines_starts.len() > context_lines_before {
                    start_line += 1;
                    before_lines_starts.pop_front();
                }
            } else if offset >= span.offset() + span.len().saturating_sub(1) {
                // We're after the end of the span, but haven't necessarily
                // started collecting end lines yet (we might still be
                // collecting context lines).
                if post_span {
                    start_column = 0;
                    if post_span_got_newline {
                        end_lines += 1;
                    } else {
                        post_span_got_newline = true;
                    }
                    if end_lines >= context_lines_after {
                        offset += 1;
                        break;
                    }
                }
            }
            current_line_start = offset + 1;
        } else if offset < span.offset() {
            start_column += 1;
        }

        if offset >= (span.offset() + span.len()).saturating_sub(1) {
            post_span = true;
            if end_lines >= context_lines_after {
                offset += 1;
                break;
            }
        }

        offset += 1;
    }

    if offset >= (span.offset() + span.len()).saturating_sub(1) {
        let starting_offset = before_lines_starts.front().copied().unwrap_or_else(|| {
            if context_lines_before == 0 {
                span.offset()
            } else {
                0
            }
        });
        Ok(if let Some(name) = name {
            miette::MietteSpanContents::new_named(
                name,
                &input[starting_offset..offset],
                (starting_offset, offset - starting_offset).into(),
                start_line,
                if context_lines_before == 0 {
                    start_column
                } else {
                    0
                },
                line_count,
            )
        } else {
            miette::MietteSpanContents::new(
                &input[starting_offset..offset],
                (starting_offset, offset - starting_offset).into(),
                start_line,
                if context_lines_before == 0 {
                    start_column
                } else {
                    0
                },
                line_count,
            )
        })
    } else {
        Err(miette::MietteError::OutOfBounds)
    }
}

impl miette::SourceCode for BytesSource {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        _context_lines_before: usize,
        _context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        let con = context_info(&self.0, span, 0, 0, self.1.as_ref().cloned())?;

        Ok(Box::new(con))
    }
}

impl BytesSource {
    pub fn new(bytes: Bytes, name: Option<String>) -> Self {
        let debag = format!("{bytes:?}");
        Self(Bytes::copy_from_slice(debag.as_bytes()), name)
    }
}

pub struct Extra<C>(PhantomData<C>);
impl<'a, C> ParserExtras<&'a [u8]> for Extra<C> {
    type Context = C;
    type Error = crate::error::Error;
}

impl<'a> aott::error::Error<&'a [u8]> for crate::error::Error {
    fn expected_eof_found(span: Range<usize>, found: <&'a [u8] as InputType>::Token) -> Self {
        Self::Ser(SerializationError::ExpectedEof {
            found,
            at: span.into(),
        })
    }

    fn expected_token_found(
        span: Range<usize>,
        expected: Vec<<&'a [u8] as InputType>::Token>,
        found: <&'a [u8] as InputType>::Token,
    ) -> Self {
        Self::Ser(SerializationError::Expected {
            expected,
            found,
            at: span.into(),
        })
    }

    fn unexpected_eof(
        span: Range<usize>,
        expected: Option<Vec<<&'a [u8] as InputType>::Token>>,
    ) -> Self {
        Self::Ser(SerializationError::UnexpectedEof {
            at: ((span.start.saturating_sub(1))..span.end).into(),
            expected,
        })
    }
}

impl<'a, C> ParserExtras<&'a str> for Extra<C> {
    type Context = C;
    type Error = crate::error::Error;
}

impl<'a> aott::error::Error<&'a str> for crate::error::Error {
    fn expected_eof_found(span: Range<usize>, found: <&'a str as InputType>::Token) -> Self {
        Self::SerStr(SerializationError::ExpectedEof {
            found,
            at: span.into(),
        })
    }

    fn expected_token_found(
        span: Range<usize>,
        expected: Vec<<&'a str as InputType>::Token>,
        found: <&'a str as InputType>::Token,
    ) -> Self {
        Self::SerStr(SerializationError::Expected {
            expected,
            found,
            at: span.into(),
        })
    }

    fn unexpected_eof(
        span: Range<usize>,
        expected: Option<Vec<<&'a str as InputType>::Token>>,
    ) -> Self {
        Self::SerStr(SerializationError::UnexpectedEof {
            at: ((span.start.saturating_sub(1))..span.end).into(),
            expected,
        })
    }
}
