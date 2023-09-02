use crate::ser::*;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::prelude::*;
use derive_more::*;

#[derive(Clone, Copy, Deref, DerefMut, Debug, Display)]
pub struct VarInt<T = i32>(pub T);

impl<T: LEB128Number> VarInt<T> {
    pub const fn max_length() -> usize {
        max_leb128_len::<T>()
    }
    pub const fn length_of(&self) -> usize {
        T::leb128_length(&self.0)
    }
}

pub trait LEB128Number: Sized {
    fn read<'parse, 'a>(input: Inp<'parse, 'a>) -> Resul<'parse, 'a, Self>;
    fn write_to<B: BufMut>(self, buf: &mut B);
    fn write(self) -> Bytes {
        let mut b = BytesMut::with_capacity(self.leb128_length());
        self.write_to(&mut b);
        b.freeze()
    }
    fn leb128_length(&self) -> usize;
}

/// Returns the length of the longest LEB128 encoding for `T`, assuming `T` is an integer type
const fn max_leb128_len<T>() -> usize {
    // The longest LEB128 encoding for an integer uses 7 bits per byte.
    (std::mem::size_of::<T>() * 8 + 6) / 7
}

/// Returns the length of the longest LEB128 encoding of all supported integer types.
const fn largest_max_leb128_len() -> usize {
    max_leb128_len::<u128>()
}

macro_rules! impl_unsigned_leb128 {
    ($($int:ty)*) => {
        $(
        impl LEB128Number for $int {
            fn read<'parse, 'a>(input: Inp<'parse, 'a>) -> Resul<'parse, 'a, $int> {
                let (mut input, byte) = any(input)?;
                if (byte & 0x80) == 0 {
                    return Ok((input, byte as $int));
                }
                let mut result = (byte & 0x7f) as $int;
                let mut shift = 7;
                loop {
                    let (inp, byte) = any(input)?;
                    input = inp;
                    if (byte & 0x80) == 0 {
                        result |= (byte as $int) << shift;
                        return Ok((input, result));
                    } else {
                        result |= ((byte & 0x7f) as $int) << shift;
                    }
                    shift += 7;
                }
            }
            fn write_to<B: BufMut>(mut self, buf: &mut B) {
                loop {
                    if self < 0x80 {
                        buf.put_u8(self as u8);

                        break;
                    } else {
                        buf.put_u8(((self & 0x7f) | 0x80) as u8);

                        self >>= 7;
                    }
                }
            }
            fn leb128_length(&self) -> usize {
                let mut i = 0;
                let mut value = *self;

                loop {
                    if value < 0x80 {
                        i += 1;
                        break;
                    } else {
                        value >>= 7;
                        i += 1;
                    }
                }

                i
            }
        })*
    };
}

macro_rules! impl_signed_leb128 {
    ($($int:ty)*) => {
        $(
            impl LEB128Number for $int {
                fn read<'parse, 'a>(mut input: Inp<'parse, 'a>) -> Resul<'parse, 'a, $int> {
                    let mut result = 0;
                    let mut shift = 0;
                    let mut byte;

                    loop {
                        byte = match input.next_or_eof() {
                            Ok(b) => b,
                            Err(eof) => return Err((input, eof))
                        };

                        result |= <$int>::from(byte & 0x7F) << shift;
                        shift += 7;

                        if (byte & 0x80) == 0 {
                            break;
                        }
                    }

                    if (shift < <$int>::BITS) && ((byte & 0x40) != 0) {
                        // sign extend
                        result |= (!0 << shift);
                    }

                    Ok((input, result))
                }
                fn write_to<B: BufMut>(mut self, buf: &mut B) {
                    loop {
                        let mut byte = (self as u8) & 0x7f;
                        self >>= 7;
                        let more = !(((self == 0) && ((byte & 0x40) == 0))
                            || ((self == -1) && ((byte & 0x40) != 0)));

                        if more {
                            byte |= 0x80; // Mark this byte to show that more bytes will follow.
                        }

                        buf.put_u8(byte);

                        if !more {
                            break;
                        }
                    }
                }
                fn leb128_length(&self) -> usize {
                    let mut value = *self;
                    let mut i = 0;

                    loop {
                        let mut byte = (value as u8) & 0x7f;
                        value >>= 7;
                        let more = !(((value == 0) && ((byte & 0x40) == 0))
                            || ((value == -1) && ((byte & 0x40) != 0)));

                        if more {
                            byte |= 0x80; // Mark this byte to show that more bytes will follow.
                        }

                        i += 1;

                        if !more {
                            break;
                        }
                    }

                    i
                }
            }
        )*
    };
}

impl_unsigned_leb128!(u16 u32 u64 u128 usize);
impl_signed_leb128!(i16 i32 i64 i128 isize);

impl<T: LEB128Number> Deserialize for VarInt<T> {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Resul<'parse, 'a, Self> {
        T::read.map(Self).parse(input)
    }
}

impl<T: LEB128Number> Serialize for VarInt<T> {
    fn serialize(&self) -> Bytes {
        self.0.write()
    }
    fn serialize_to(&self, buf: &mut BytesMut) {
        self.0.write_to(buf)
    }
}
