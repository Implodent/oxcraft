use derive_more::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deref, DerefMut, Into, From, Debug, Display)]
pub struct VarInt(pub i32);

impl VarInt {
    const SEGMENT_BITS: u8 = 0x7F;
    const CONTINUE_BIT: u8 = 0x80;
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut iter = bytes.iter().copied();
        Self::from_bytes_iter(&mut iter)
    }
    pub fn from_bytes_iter(bytes: &mut impl Iterator<Item = u8>) -> Option<Self> {
        match bytes.try_fold((0i32, 0u8), |(mut value, mut position), byte| {
            value |= ((byte & Self::SEGMENT_BITS) << position) as i32;

            if (byte & Self::CONTINUE_BIT) == 0 {
                // does not have continue bit
                return Err(Some((value, position)));
            }

            position += 7;

            if position >= 32 {
                Err(None)
            } else {
                Ok((value, position))
            }
        }) {
            Ok((value, _)) | Err(Some((value, _))) => Some(VarInt(value)),
            Err(None) => None,
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut value = self.0;
        let mut bytes = vec![];
        loop {
            if (value & !Self::SEGMENT_BITS as i32) == 0 {
                bytes.push(value as u8);
                break;
            }

            bytes.push(((value & Self::SEGMENT_BITS as i32) | Self::CONTINUE_BIT as i32) as u8);

            // Note: >>> means that the sign bit is shifted with the rest of the number rather than being left alone
            value = value.rotate_right(7);
        }
        bytes
    }
    pub fn len(&self) -> usize {
        let mut value = self.0;
        let mut length = 0usize;
        loop {
            if (value & !Self::SEGMENT_BITS as i32) == 0 {
                length += 1;
                break;
            }
            length += 1;
            value = value.rotate_right(7);
        }
        length
    }
    // clippy why
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl Serialize for VarInt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let b = self.to_bytes();
        serializer.serialize_bytes(&b)
    }
}

impl<'de> Deserialize<'de> for VarInt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'v> serde::de::Visitor<'v> for V {
            type Value = VarInt;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "expecting VarInt")
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                VarInt::from_bytes(v).ok_or_else(|| E::custom("VarInt too big"))
            }
        }
        deserializer.deserialize_bytes(V)
    }
}
