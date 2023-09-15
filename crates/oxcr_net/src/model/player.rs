use bevy::prelude::{Component};
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