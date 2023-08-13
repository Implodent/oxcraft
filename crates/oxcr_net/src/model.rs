pub mod packets;
pub mod varint;

#[cfg(not(target_pointer_width = "32"))]
pub const MAX_PACKET_DATA: usize = 0x1FFFFF;
#[cfg(target_pointer_width = "32")]
compile_error!("bruh");
