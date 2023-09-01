use crate::ser::*;
use ::bytes::Bytes;
use aott::prelude::*;
use derive_more::*;

#[derive(Clone, Copy, Deref, DerefMut, Into, From, Debug, Display)]
pub struct VarInt<T: LEB128Number = i32>(pub T);
impl<T: LEB128Number> VarInt<T> {
    pub fn max_length() -> usize {
        max_leb128_len::<T>()
    }
}

pub trait LEB128Number {
    fn read<'parse, 'a>(
        input: Input<'parse, &'a [u8], extra::Err<&'a [u8]>>,
    ) -> IResult<'parse, &'a [u8], extra::Err<&'a [u8]>, Self>;
    fn write(self) -> Bytes;
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
            fn read<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, $int> {
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
        }
        fn write(mut self) -> Bytes {
            let mut out = [MaybeUninit::uninit(); max_leb128_len::<$int>()];
            let mut i = 0;

            loop {
                if self < 0x80 {
                    unsafe {
                        *out.get_unchecked_mut(i).as_mut_ptr() = self as u8;
                    }

                    i += 1;
                    break;
                } else {
                    unsafe {
                        *out.get_unchecked_mut(i).as_mut_ptr() = ((self & 0x7f) | 0x80) as u8;
                    }

                    self >>= 7;
                    i += 1;
                }
            }

            Bytes::from(unsafe { ::std::mem::MaybeUninit::slice_assume_init_ref(&out.get_unchecked(..i)) })
        })*
    };
}

macro_rules! impl_signed_leb128 {
    ($($int:ty)*) => {
        $(
            impl LEB128Number for $int {
                fn read<'parse, 'a>(mut input: Inp<'parse, 'a>) -> Res<'parse, 'a, $int> {
                    let mut result = 0;
                    let mut shift = 0;
                    let mut byte;

                    loop {
                        let (inp, by) = any(input)?;
                        input = inp;
                        byte = by;

                        result |= <$int_ty>::from(byte & 0x7F) << shift;
                        shift += 7;

                        if (byte & 0x80) == 0 {
                            break;
                        }
                    }

                    if (shift < <$int_ty>::BITS) && ((byte & 0x40) != 0) {
                        // sign extend
                        result |= (!0 << shift);
                    }

                    Ok((input, result))
                }
                fn write(self) -> Bytes {
                    let mut out = [MaybeUninit::uninit(); max_leb128_len::<$int>()];
                    let mut i = 0;

                    loop {
                        let mut byte = (self as u8) & 0x7f;
                        self >>= 7;
                        let more = !(((self == 0) && ((byte & 0x40) == 0))
                            || ((self == -1) && ((byte & 0x40) != 0)));

                        if more {
                            byte |= 0x80; // Mark this byte to show that more bytes will follow.
                        }

                        unsafe {
                            *out.get_unchecked_mut(i).as_mut_ptr() = byte;
                        }

                        i += 1;

                        if !more {
                            break;
                        }
                    }

                    Bytes::from(unsafe { ::std::mem::MaybeUninit::slice_assume_init_ref(&out.get_unchecked(..i)) })
                }
            }
        )*
    };
}

impl_unsigned_leb128!(u16 u32 u64 u128 usize);
impl_signed_leb128!(i16 i32 i64 i128 isize);

impl<T: LEB128Number> Deserialize for VarInt<T> {
    fn deserialize<'parse, 'a>(input: Inp<'parse, 'a>) -> Res<'parse, 'a, Self> {
        T::read.map(Self).parse(input)
    }
}

impl<T: LEB128Number> Serialize for VarInt<T> {
    fn serialize(&self) -> Bytes {
        self.0.write()
    }
}
