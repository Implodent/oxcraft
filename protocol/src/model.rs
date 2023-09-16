pub mod chat;
pub mod packets;
mod varint;
use aott::primitive::one_of;
use bytes::BufMut;
use std::ptr;
pub use varint::*;

use crate::ser::{Deserialize, Serialize};
pub mod item;

pub const MAX_PACKET_DATA: usize = 0x1FFFFF;
pub const PROTOCOL_VERSION: i32 = 763;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Difficulty {
    #[default]
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl Serialize for Difficulty {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_u8(*self as _)
    }
}

impl Deserialize for Difficulty {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], crate::ser::Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, crate::ser::Extra<Self::Context>> {
        let byte = one_of([0x0, 0x1, 0x2, 0x3])(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}
