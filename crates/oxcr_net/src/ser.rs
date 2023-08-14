use bytes::{BufMut, BytesMut};
use serde::Serializer;

#[derive(thiserror::Error, Debug)]
pub enum Error {}

type Result<T = ()> = core::result::Result<T, Error>;
