pub mod packets;
mod varint;
pub use varint::*;

pub const MAX_PACKET_DATA: usize = 0x1FFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}
