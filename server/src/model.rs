use bevy::prelude::Component;
use oxcr_protocol::{model::packets::play::GameMode, ser::*, uuid::Uuid};

#[derive(Component, Debug)]
pub struct Player {
    pub name: FixedStr<16, YesSync>,
    pub uuid: Uuid,
    pub game_mode: GameMode,
}
