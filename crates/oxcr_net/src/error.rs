#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("duplicate player IP")]
    DupePlayer,
    #[error("IO error: {_0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {_0}")]
    Ser(#[from] aott::extra::Simple<u8>),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
