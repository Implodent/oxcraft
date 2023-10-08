#![allow(dead_code)]
use crate::{
    nbt::Nbt,
    nsfr::{i12, i26},
};
use derive_more::*;
use indexmap::IndexMap;
use miette::SourceSpan;
use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, Range},
    rc::Rc,
    sync::Arc,
};
use uuid::Uuid;

use crate::model::VarInt;
use ::bytes::{BufMut, Bytes, BytesMut};
pub use aott::prelude::parser;
pub use aott::prelude::Parser;
use aott::{pfn_type, prelude::*, text::CharError};
use tracing::debug;

pub trait Deserialize: Sized {
    type Context = ();

    fn deserialize<'a>(
        input: &mut Input<&'a [u8], Extra<Self::Context>>,
    ) -> PResult<&'a [u8], Self, Extra<Self::Context>>;
}

pub trait Serialize {
    fn serialize(&self) -> Result<::bytes::Bytes, crate::error::Error> {
        try {
            let mut b = BytesMut::new();
            self.serialize_to(&mut b)?;
            b.freeze()
        }
    }
    /// Serializes `self` to the given buffer.
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error>;
}

pub fn any_of<T: Debug>(things: &[T]) -> String {
    match things {
        [el] => format!("{el:?}"),
        elements => format!("any of {elements:?}"),
    }
}

pub fn any_of_display<T: Display>(things: &[T]) -> String {
    match things {
        [el] => format!("{el}"),
        elements => format!(
            "any of [{}]",
            elements
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(", ")
        ),
    }
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum SerializationError<Item: Debug + Display> {
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
    #[error("filter failed in {location}, while checking {token}")]
    #[diagnostic(code(aott::error::filter_failed))]
    FilterFailed {
        #[label = "this is the token that didn't pass"]
        at: SourceSpan,
        location: &'static core::panic::Location<'static>,
        token: Item,
    },
    #[error("expected keyword {keyword}")]
    #[diagnostic(code(aott::text::error::expected_keyword))]
    ExpectedKeyword {
        #[label = "the keyword here is {found}"]
        at: SourceSpan,
        keyword: String,
        found: String,
    },
    #[error("expected digit of radix {radix}, found {found}")]
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

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("{}", any_of_display(.errors))]
pub struct WithSource<Item: Debug + Display + 'static> {
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

impl<'a> Error<&'a [u8]> for crate::error::Error {
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

    fn filter_failed(
        span: Range<usize>,
        location: &'static core::panic::Location<'static>,
        token: <&'a [u8] as InputType>::Token,
    ) -> Self {
        Self::Ser(SerializationError::FilterFailed {
            at: span.into(),
            location,
            token,
        })
    }
}

impl<'a, C> ParserExtras<&'a str> for Extra<C> {
    type Context = C;
    type Error = crate::error::Error;
}

impl<'a> Error<&'a str> for crate::error::Error {
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

    fn filter_failed(
        span: Range<usize>,
        location: &'static core::panic::Location<'static>,
        token: <&'a str as InputType>::Token,
    ) -> Self {
        Self::SerStr(SerializationError::FilterFailed {
            at: span.into(),
            location,
            token,
        })
    }
}

impl CharError<char> for crate::error::Error {
    fn expected_digit(span: Range<usize>, radix: u32, got: char) -> Self {
        Self::SerStr(SerializationError::ExpectedDigit {
            at: span.into(),
            radix,
            found: got,
        })
    }

    fn expected_ident_char(span: Range<usize>, got: char) -> Self {
        Self::SerStr(SerializationError::ExpectedIdent {
            at: span.into(),
            found: got,
        })
    }

    fn expected_keyword<'a, 'b: 'a>(
        span: Range<usize>,
        keyword: &'b <char as text::Char>::Str,
        actual: &'a <char as text::Char>::Str,
    ) -> Self {
        Self::SerStr(SerializationError::ExpectedKeyword {
            at: span.into(),
            keyword: keyword.to_owned(),
            found: actual.to_owned(),
        })
    }
}

pub type Resul<'a, T, C = ()> = PResult<&'a [u8], T, Extra<C>>;

#[parser(extras = "Extra<C>")]
pub fn deser_cx<T: Deserialize<Context = ()>, C>(input: &[u8]) -> T {
    no_context(T::deserialize)(input)
}

#[parser(extras = "Extra<C>")]
pub fn deser<T: Deserialize<Context = C>, C>(input: &[u8]) -> T {
    T::deserialize(input)
}

#[track_caller]
pub fn no_context<
    I: InputType,
    O,
    E: ParserExtras<I, Context = ()>,
    EE: ParserExtras<I, Context = C, Error = E::Error>,
    C,
    P: Parser<I, O, E>,
>(
    parser: P,
) -> pfn_type!(I, O, EE) {
    #[track_caller]
    move |input| {
        let mut inp = Input {
            offset: input.offset,
            input: input.input,
            cx: &(),
        };
        let value = parser.parse_with(&mut inp)?;
        input.offset = inp.offset;
        Ok(value)
    }
}

#[track_caller]
pub fn with_context<
    I: InputType,
    O,
    E: ParserExtras<I, Context = C2>,
    EE: ParserExtras<I, Context = C1, Error = E::Error>,
    C1,
    C2,
    P: Parser<I, O, E>,
>(
    parser: P,
    context: C2,
) -> pfn_type!(I, O, EE) {
    #[track_caller]
    move |input| {
        let mut inp = Input {
            offset: input.offset,
            input: input.input,
            cx: &context,
        };
        let value = parser.parse_with(&mut inp)?;
        input.offset = inp.offset;
        Ok(value)
    }
}

#[parser(extras = E)]
pub fn slice_till_end<'a, I: SliceInput<'a>, E: ParserExtras<I>>(input: I) -> I::Slice {
    Ok(input.input.slice_from(input.offset..))
}

impl<const N: usize, Sy: Syncable> Deserialize for FixedStr<N, Sy> {
    type Context = ();

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        // get length, must be 0 <= length <= N
        let VarInt(length) = VarInt::<i32>::deserialize(input)?;
        debug!(%length, expected_length=%N, "[fixedstr] checking length");
        assert!(length >= 0);
        let length = length as usize;
        assert!(length <= N);
        let string =
            std::str::from_utf8(input.input.slice((input.offset)..(input.offset + length)))
                .expect("invalid utf8. that's a skill issue ngl");
        input.offset += length;

        // SAFETY: checked length being <= N, can unwrap_unchecked here.
        Ok(unsafe { FixedStr::from_string(string).unwrap_unchecked() })
    }
}

impl<const N: usize, Sy: Syncable> Serialize for FixedStr<N, Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            let n: i32 = self.len().try_into().unwrap();
            let ln = VarInt(n);
            buf.reserve(ln.length_of() + self.len());
            ln.serialize_to(buf)?;
            buf.put_slice(self.as_bytes())
        }
    }
}

#[derive(Clone, Debug)]
pub struct FixedStr<const N: usize, Sy: Syncable = YesSync> {
    inner: Sy::RefC<str>,
}

impl<const N: usize, Sy: Syncable> std::fmt::Display for FixedStr<N, Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self.inner.deref(), f)
    }
}
impl<const N: usize, Sy: Syncable> FixedStr<N, Sy> {
    pub fn from_string(string: &str) -> Option<Self> {
        if string.len() > N {
            None
        } else {
            Some(Self {
                inner: Sy::refc_from_str(string),
            })
        }
    }
}

impl<const N: usize, Sy: Syncable> Deref for FixedStr<N, Sy> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.inner
    }
}

pub trait Syncable {
    const SYNC: bool;
    type RefC<T: ?Sized>: Clone + Deref<Target = T> + Sized;

    fn refc_new<T>(t: T) -> Self::RefC<T>;
    fn refc_from_str(s: &str) -> Self::RefC<str>;
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]>;
    fn refc_from_array<const N: usize, T>(array: [T; N]) -> Self::RefC<[T]>;
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct YesSync;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoSync;
impl Syncable for YesSync {
    const SYNC: bool = true;
    type RefC<T: ?Sized> = Arc<T>;
    fn refc_new<T>(t: T) -> Self::RefC<T> {
        Arc::new(t)
    }
    fn refc_from_str(s: &str) -> Self::RefC<str> {
        Arc::from(s)
    }
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]> {
        Arc::from(slice)
    }
    fn refc_from_array<const N: usize, T>(array: [T; N]) -> Self::RefC<[T]> {
        Arc::new(array)
    }
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]> {
        Arc::from_iter(iter)
    }
}
impl Syncable for NoSync {
    const SYNC: bool = false;
    type RefC<T: ?Sized> = Rc<T>;
    fn refc_new<T>(t: T) -> Self::RefC<T> {
        Rc::new(t)
    }
    fn refc_from_str(s: &str) -> Self::RefC<str> {
        Rc::from(s)
    }
    fn refc_from_slice<T: Clone>(slice: &[T]) -> Self::RefC<[T]> {
        Rc::from(slice)
    }
    fn refc_from_array<const N: usize, T>(array: [T; N]) -> Self::RefC<[T]> {
        Rc::new(array)
    }
    fn refc_from_iter<T: Clone>(iter: impl IntoIterator<Item = T>) -> Self::RefC<[T]> {
        Rc::from_iter(iter)
    }
}

impl<'a> Serialize for &'a str {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            VarInt::<i32>(self.len().try_into().unwrap()).serialize_to(buf)?;
            buf.put_slice(self.as_bytes());
        }
    }
}

#[derive(Clone, Copy, Deref, DerefMut, Display)]
pub struct Json<T>(pub T);
impl<T: Debug + serde::Serialize> Debug for Json<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Json({:?}) = {}",
            self.0,
            serde_json::to_string_pretty(&self.0).unwrap()
        )
    }
}

impl<T: serde::Serialize> Serialize for Json<T> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            let s = serde_json::to_string(&self.0).expect("json fail");
            s.as_str().serialize_to(buf)?
        }
    }
}

impl<T: for<'de> serde::Deserialize<'de>> Deserialize for Json<T> {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let VarInt(length) = VarInt::<i32>::deserialize(input)?;
        assert!(length >= 0);
        let length = length as usize;
        let slic = input.input.slice(input.offset..input.offset + length);
        match serde_json::from_slice::<T>(slic) {
            Ok(j) => Ok(Self(j)),
            Err(e) => Err(crate::error::Error::Json(e)),
        }
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    type Context = T::Context;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let exists = one_of([0x0, 0x1]).map(|t| t == 0x1).parse_with(input)?;
        if exists {
            let value = T::deserialize(input)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

impl<T: Serialize> Serialize for Option<T> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            match self {
                Some(val) => {
                    buf.put_u8(0x1);
                    val.serialize_to(buf)?
                }
                None => buf.put_u8(0x0),
            }
        }
    }
}

impl Serialize for Uuid {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try { buf.put_u128(self.as_u128()) }
    }
}

impl Deserialize for Uuid {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        Ok(Self::from_bytes(take_exact().parse_with(input)?))
    }
}

#[derive(Debug, Deref, Clone, PartialEq, Eq)]
#[deref(forward)]
pub struct Array<T, Sy: Syncable = YesSync>(Sy::RefC<[T]>);

impl<T, Sy: Syncable> Array<T, Sy> {
    pub fn empty() -> Self {
        Self(Sy::refc_from_array([]))
    }
    pub fn new_from_array<const N: usize>(array: [T; N]) -> Self {
        Self(Sy::refc_from_array(array))
    }
    pub fn new(slice: &[T]) -> Self
    where
        T: Clone,
    {
        Self(Sy::refc_from_slice(slice))
    }
}

impl<T: Serialize, Sy: Syncable> Serialize for Array<T, Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            VarInt::<i32>(self.len().try_into().unwrap()).serialize_to(buf)?;
            for item in self.iter() {
                item.serialize_to(buf)?;
            }
        }
    }
}

impl<T: Clone, Sy: Syncable> FromIterator<T> for Array<T, Sy> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Sy::refc_from_iter(iter))
    }
}

impl<T: Clone + Deserialize, Sy: Syncable> Deserialize for Array<T, Sy> {
    type Context = <T as Deserialize>::Context;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let VarInt::<i32>(length) = deser_cx(input)?;
        debug_assert!(length >= 0);
        let length = length as usize;

        T::deserialize
            .repeated_custom::<Self>()
            .exactly(length)
            .parse_with(input)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MaybeHeap<'a, T: ?Sized, Sy: Syncable = YesSync> {
    Heap(Sy::RefC<T>),
    Ref(&'a T),
}

impl<'a, T: Clone, Sy: Syncable> Clone for MaybeHeap<'a, T, Sy> {
    fn clone(&self) -> Self {
        match self {
            Self::Heap(heap) => Self::Heap(heap.clone()),
            Self::Ref(r) => Self::Heap(Sy::refc_new((*r).clone())),
        }
    }
}
impl<'a, Sy: Syncable> Clone for MaybeHeap<'a, str, Sy> {
    fn clone(&self) -> Self {
        match self {
            Self::Heap(heap) => Self::Heap(heap.clone()),
            Self::Ref(r) => Self::Ref(*r),
        }
    }
}

impl<'a, T: ?Sized, Sy: Syncable> Deref for MaybeHeap<'a, T, Sy> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Heap(heap) => heap.deref(),
            Self::Ref(r) => r,
        }
    }
}

#[derive(Clone)]
pub struct Identifier<Sy: Syncable = YesSync>(pub Namespace, pub MaybeHeap<'static, str, Sy>);
impl<Sy: Syncable> PartialEq<Self> for Identifier<Sy> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1.deref() == other.1.deref()
    }
}
impl<Sy: Syncable> Eq for Identifier<Sy> {}
impl<Sy: Syncable> std::fmt::Debug for Identifier<Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Identifier({:?}, {:?})", self.0, self.1.deref())
    }
}
impl<Sy: Syncable> Display for Identifier<Sy> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.0, self.1.deref())
    }
}

impl<Sy: Syncable> Identifier<Sy> {
    pub const MINECRAFT_BRAND: Self = Self(Namespace::Minecraft, MaybeHeap::Ref("brand"));

    pub const fn new_static(namespace: Namespace, value: &'static str) -> Self {
        Self(namespace, MaybeHeap::Ref(value))
    }
    pub fn new(namespace: Namespace, value: &str) -> Self {
        Self(namespace, MaybeHeap::Heap(Sy::refc_from_str(value)))
    }

    #[parser(extras = "Extra<()>")]
    fn parse(input: &str) -> Self {
        (
            aott::text::ascii::ident.map(|namespace| match namespace {
                "minecraft" => Namespace::Minecraft,
                anything_else => Namespace::Custom(Cow::Owned(anything_else.to_owned())),
            }),
            just(":").ignored(),
            aott::text::ascii::ident.map(Sy::refc_from_str),
        )
            .map(|(namespace, (), value)| Self(namespace, MaybeHeap::Heap(value)))
            .parse_with(input)
    }
}

impl<Sy: Syncable> Serialize for Identifier<Sy> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            let formatted = format!("{}:{}", self.0, &*self.1);
            formatted.as_str().serialize_to(buf)?
        }
    }
}
impl<Sy: Syncable> Deserialize for Identifier<Sy> {
    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let FixedStr::<32767, Sy> { inner } = FixedStr::deserialize(input)?;
        Self::parse.parse(inner.deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum Namespace {
    #[display(fmt = "minecraft")]
    Minecraft,
    #[display(fmt = "{_0}")]
    Custom(Cow<'static, str>),
}

pub trait Endian {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Big;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Little;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Native;

impl Endian for Big {}
impl Endian for Little {}
impl Endian for Native {}

/// A type for serializing and deserializing numbers from bytes.
/// # Examples
/// ```
/// # use aott::prelude::*;
/// # use oxcr_protocol::ser::*;
/// assert_eq!(Number::<u8, Big>::deserialize.parse(&[0x63]).unwrap(), Number(99u8, Big));
/// ```
#[derive(Copy, Clone, Deref, DerefMut, Debug, Display, PartialEq, Eq)]
#[display(bound = "N: core::fmt::Display")]
#[display(fmt = "{_0}")]
pub struct Number<N, E: Endian>(
    #[deref]
    #[deref_mut]
    pub N,
    pub E,
);

macro_rules! number_impl {
        ($($num:ty)*) => {
                $(
                impl Deserialize
                        for Number<$num, Big>
                {
                        #[track_caller]
                        fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                        where
                                Self: Sized,
                        {
                                Ok(Self(
                                        <$num>::from_be_bytes(
                                                take_exact::<{ core::mem::size_of::<$num>() }>()
                                                        .parse_with(input)?,
                                        ),
                                        Big,
                                ))
                        }
                }
                impl Deserialize
                        for Number<$num, Little>
                {
                        fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                        where
                                Self: Sized,
                        {
                                Ok(Self(
                                        <$num>::from_le_bytes(
                                                take_exact::<{ core::mem::size_of::<$num>() }>()
                                                        .parse_with(input)?,
                                        ),
                                        Little,
                                ))
                        }
                }
                impl Deserialize for $num {
                    #[track_caller]
                    fn deserialize<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> PResult<&'a [u8], Self, Extra<()>>
                    where
                            Self: Sized,
                    {
                            Ok(
                                    <$num>::from_be_bytes(
                                            take_exact::<{ core::mem::size_of::<$num>() }>()
                                                    .parse_with(input)?,
                                    ),
                            )
                    }
                }
                impl Serialize for $num {
                    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> { try {
                        buf.put(&self.to_be_bytes()[..])
                    }}
                }
            )*
        };
}

number_impl![u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64];

impl Serialize for bool {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            buf.put_u8(*self as u8);
        }
    }
}

impl Deserialize for bool {
    #[parser(extras = "Extra<Self::Context>")]
    #[track_caller]
    fn deserialize(input: &[u8]) -> Self {
        Ok(one_of([0x0, 0x1])(input)? == 0x1)
    }
}

/// # Layout
/// x as a 26-bit integer, followed by z as a 26-bit integer, followed by y as a 12-bit integer (all signed, two's complement).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub x: i26,
    pub z: i26,
    pub y: i12,
}

impl Serialize for Position {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try {
            let pos = (i64::from(self.x) & 0x3ffffff) << 38
                | (i64::from(self.z) & 0x3ffffff) << 12
                | (i64::from(self.y) & 0xfff);
            buf.put_i64(pos);
        }
    }
}

impl Deserialize for Position {
    type Context = ();

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        let val = aott::bytes::number::big::i64(input)?;

        unsafe {
            Ok(Self {
                x: i26::new_unchecked((val >> 38).try_into().unwrap_unchecked()),
                z: i26::new_unchecked((val << 26 >> 38).try_into().unwrap_unchecked()),
                y: i12::new_unchecked((val << 52 >> 52).try_into().unwrap_unchecked()),
            })
        }
    }
}

pub macro serialize($ty:ty => [$($field:ident),*$(,)?]) {
    impl crate::ser::Serialize for $ty {
        fn serialize_to(&self, buf: &mut ::bytes::BytesMut) -> Result<(), crate::error::Error> {
            try {
                $(self.$field.serialize_to(buf)?;)*
            }
        }
    }
}
pub macro deserialize($(|$context:ty|)? $ty:ty => [$($(|$cx:ty|)?$field:ident),*$(,)?]) {
    impl crate::ser::Deserialize for $ty {
        type Context = deserialize_field!($(|$context|)?);

        #[parser(extras = "crate::ser::Extra<Self::Context>")]
        fn deserialize(input: &[u8]) -> Self {
            Ok(Self {
                $($field: crate::ser::deserialize_field!($(|$cx:ty|)?$field, input)?,)*
            })
        }
    }
}

macro deserialize_field {
    (|$cx:ty| $field:ident, $input:ident) => {
        crate::ser::deser($input)
    },
    ($field:ident, $input:ident) => {
        crate::ser::deser_cx($input)
    },
    (|$context:ty|) => {
        $context
    },
    () => {
        ()
    }
}

pub macro impl_ser($(|$context:ty|)? $ty:ty => [$($(|$cx:ty|)?$field:ident),*$(,)?]) {
    crate::ser::deserialize!($(|$context|)? $ty => [$($(|$cx|)?$field,)*]);
    crate::ser::serialize!($ty => [$($field,)*]);
}

impl Serialize for Bytes {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        try { buf.put_slice(&self) }
    }
}

impl Deserialize for Bytes {
    #[track_caller]
    fn deserialize<'a>(
        input: &mut Input<&'a [u8], Extra<Self::Context>>,
    ) -> PResult<&'a [u8], Self, Extra<Self::Context>> {
        Ok(Bytes::copy_from_slice(
            input.input.slice_from(input.offset..),
        ))
    }
}

impl Serialize for IndexMap<String, Nbt> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        Nbt::serialize_compound(self, buf)
    }
}

impl Deserialize for IndexMap<String, Nbt> {
    fn deserialize<'a>(
        input: &mut Input<&'a [u8], Extra<Self::Context>>,
    ) -> PResult<&'a [u8], Self, Extra<Self::Context>> {
        Nbt::compound(input)
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Zlib;
#[derive(Default, Clone, Copy, Debug)]
pub struct Zstd;
#[derive(Default, Clone, Copy, Debug)]
pub struct Gzip;
pub trait Compression {
    fn encode(data: &[u8]) -> Result<Bytes, crate::error::Error>;
    fn encode_serialize<T: Serialize>(
        thing: &T,
        buf: &mut BytesMut,
    ) -> Result<(), crate::error::Error>;
    fn decode(data: &[u8]) -> Result<Bytes, crate::error::Error>;
}
impl Compression for Zlib {
    fn encode(data: &[u8]) -> Result<Bytes, crate::error::Error> {
        use std::io::Write;
        let mut enc = flate2::write::ZlibEncoder::new(
            BytesMut::new().writer(),
            flate2::Compression::default(),
        );
        enc.write_all(data)?;
        Ok(enc.finish()?.into_inner().freeze())
    }

    fn encode_serialize<T: Serialize>(
        thing: &T,
        buf: &mut BytesMut,
    ) -> Result<(), crate::error::Error> {
        use std::io::{self, Write};

        struct WriterMut<'a, B: BufMut>(&'a mut B);
        impl<'a, B: BufMut> Write for WriterMut<'a, B> {
            fn write(&mut self, src: &[u8]) -> io::Result<usize> {
                let n = std::cmp::min(self.0.remaining_mut(), src.len());

                self.0.put(&src[0..n]);
                Ok(n)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut enc =
            flate2::write::ZlibEncoder::new(WriterMut(buf), flate2::Compression::default());

        let mut bmut = BytesMut::new();
        thing.serialize_to(&mut bmut)?;
        enc.write_all(&bmut)?;

        enc.finish()?;

        Ok(())
    }

    fn decode(data: &[u8]) -> Result<Bytes, crate::error::Error> {
        use std::io::Read;
        let mut dec = flate2::read::ZlibDecoder::new(std::io::Cursor::new(data));
        let mut buf = Vec::new();
        dec.read_to_end(&mut buf)?;
        Ok(Bytes::from(buf))
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Compress<T, C: Compression = Zlib>(pub T, pub C);

impl<T: Serialize, C: Compression> Serialize for Compress<T, C> {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        C::encode_serialize(&self.0, buf)
    }
}

impl<T: Deserialize<Context = ()>, C: Compression + Default> Compress<T, C> {
    pub fn decompress(buffer: &[u8]) -> Result<Self, crate::error::Error> {
        let buffer = C::decode(buffer)?;
        Ok(Self(T::deserialize.parse(&buffer)?, C::default()))
    }
}

impl<'a, T: Serialize + ?Sized> Serialize for &'a T {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        (**self).serialize_to(buf)
    }
}

impl<T1: Serialize, T2: Serialize> Serialize for (T1, T2) {
    fn serialize_to(&self, buf: &mut BytesMut) -> Result<(), crate::error::Error> {
        self.0.serialize_to(buf)?;
        self.1.serialize_to(buf)?;
        Ok(())
    }
}
