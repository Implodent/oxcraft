use std::ptr;

use aott::primitive::one_of;
use bevy::prelude::Component;
use bytes::BufMut;
use uuid::Uuid;

use crate::ser::*;

#[derive(Component, Debug)]
pub struct Player {
    pub name: FixedStr<16, YesSync>,
    pub uuid: Uuid,
    pub game_mode: GameMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl Serialize for GameMode {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_u8(*self as u8);
    }
}

impl Deserialize for GameMode {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let byte = one_of([0x0, 0x1, 0x2, 0x3])(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum PreviousGameMode {
    Undefined = -1,
    Normal(GameMode),
}
impl Serialize for PreviousGameMode {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_i8(match self {
            Self::Undefined => -1,
            Self::Normal(gamemode) => *gamemode as i8,
        });
    }
}
impl Deserialize for PreviousGameMode {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let byte = aott::bytes::number::big::i8
            .filter(|g| (-1..=3).contains(g))
            .parse_with(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}
