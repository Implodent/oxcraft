use crate::ser::*;
use ::bytes::{BufMut, Bytes, BytesMut};
use aott::prelude::*;
use derive_more::*;

#[derive(Clone, Copy, Deref, DerefMut, Debug, Display, PartialEq, Eq)]
pub struct VarInt<T = i32>(pub T);

impl<T: LEB128Number> VarInt<T> {
    pub fn length_of(&self) -> usize {
        T::leb128_length(&self.0)
    }
}

pub trait LEB128Number: Sized + Copy {
    fn read<'a>(input: &mut Input<&'a [u8], Extra<()>>) -> Resul<'a, Self>;
    fn write_to<B: BufMut>(self, buf: &mut B);
    fn write(self) -> Bytes {
        let mut b = BytesMut::with_capacity(self.leb128_length());
        self.write_to(&mut b);
        b.freeze()
    }
    fn leb128_length(&self) -> usize;
}

macro_rules! leb_impl_signed {
    ($signed:ty, $unsigned:ty, $max:literal) => {
        impl LEB128Number for $signed {
            #[parser(extras = "Extra<()>")]
            fn read(input: &[u8]) -> Self {
                let mut result: $signed = 0;
                let mut shift: $signed = 0;

                let mut byte: u8;
                loop {
                    byte = input.next()?;

                    result |= ((byte & 0x7fu8) as $signed) << shift;

                    if (byte & 0x80) == 0 {
                        break;
                    }
                    shift += 7;

                    if shift >= $max {
                        return Err(crate::error::Error::VarIntTooBig);
                    }
                }

                Ok(result)
            }

            fn write_to<B: BufMut>(mut self, buf: &mut B) {
                loop {
                    if (self & !0x7f) == 0 {
                        buf.put_u8(self as u8);
                        break;
                    }

                    buf.put_u8(((self as u8) & 0x7f) | 0x80);

                    self = <$signed>::from_be_bytes(
                        ((<$unsigned>::from_be_bytes(self.to_be_bytes())) >> 7).to_be_bytes(),
                    );
                }
            }

            fn leb128_length(&self) -> usize {
                let mut this = *self;
                let mut l = 0usize;
                while (this & -128) != 0 {
                    l += 1;
                    this = <$signed>::from_be_bytes(
                        ((<$unsigned>::from_be_bytes(this.to_be_bytes())) >> 7).to_be_bytes(),
                    );
                }
                l + 1
            }
        }
    };
}

leb_impl_signed!(i32, u32, 32);
leb_impl_signed!(i64, u64, 64);

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vi<T: LEB128Number + std::fmt::Debug + std::cmp::PartialEq>(bytes: &[u8], value: T) {
        assert_eq!(
            VarInt::<T>::deserialize.parse(bytes).unwrap(),
            VarInt(value)
        );
        assert_eq!(VarInt(value).serialize(), bytes);
    }

    #[test]
    fn test_i32() {
        test_vi(&[0x00], 0);
        test_vi(&[0x01], 1);
        test_vi(&[0x02], 2);
        test_vi(&[0x7f], 127);
        test_vi(&[0x80, 0x01], 128);
        test_vi(&[0xff, 0x01], 255);
        test_vi(&[0xdd, 0xc7, 0x01], 25565);
        test_vi(&[0xff, 0xff, 0x7f], 2097151);
    }
}

impl<T: LEB128Number> Deserialize for VarInt<T> {
    #[parser(extras = "Extra<()>")]
    fn deserialize(input: &[u8]) -> Self {
        T::read.map(Self).parse_with(input)
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
