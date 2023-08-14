use bytes::{BufMut, BytesMut};
use serde::Serializer;

#[derive(thiserror::Error, Debug)]
pub enum Error {}

type Result<T = ()> = core::result::Result<T, Error>;

struct SerializePacket(BytesMut);

impl<'a> Serializer for &'a mut SerializePacket {
    type Ok = ();
    type Error = Error;

    fn serialize_bytes(self, v: &[u8]) -> Result {
        self.0.extend(v);
        Ok(())
    }

    fn serialize_bool(self, v: bool) -> Result {
        self.0.put_u8(if v { 0x1u8 } else { 0x0u8 });
        Ok(())
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
