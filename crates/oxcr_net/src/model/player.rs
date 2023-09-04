use bevy::prelude::Component;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum PreviousGameMode {
    Undefined = -1,
    Normal(GameMode),
}
