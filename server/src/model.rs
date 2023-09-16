use bevy::prelude::{Component, Resource};
use oxcr_protocol::{
    model::{packets::play::GameMode, Difficulty},
    ser::*,
    uuid::Uuid,
};

#[derive(Component, Debug)]
pub struct Player {
    pub name: FixedStr<16, YesSync>,
    pub uuid: Uuid,
    pub game_mode: GameMode,
}

#[derive(Debug, Resource, Default, Clone, Copy)]
pub struct DifficultySetting {
    pub difficulty: Difficulty,
    pub is_locked: bool,
}
