use bevy::prelude::{Bundle, Component, Resource};
use oxcr_protocol::{
    model::{packets::play::GameMode, Difficulty},
    ser::*,
    uuid::Uuid,
};

#[derive(Component, Debug)]
pub struct Player;

#[derive(Component, Debug)]
pub struct PlayerName(pub FixedStr<16, YesSync>);

#[derive(Component, Debug)]
pub struct PlayerUuid(pub Uuid);

#[derive(Component, Debug)]
pub struct PlayerGameMode(pub GameMode);

#[derive(Bundle, Debug)]
pub struct PlayerBundle {
    pub player_marker: Player,
    pub name: PlayerName,
    pub uuid: PlayerUuid,
    pub game_mode: PlayerGameMode,
}

#[derive(Debug, Resource, Default, Clone, Copy)]
pub struct DifficultySetting {
    pub difficulty: Difficulty,
    pub is_locked: bool,
}
