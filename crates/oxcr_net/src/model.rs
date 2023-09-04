pub mod chat;
pub mod packets;
pub mod player;
mod varint;
pub use varint::*;
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
