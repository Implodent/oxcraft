#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("duplicate player IP")]
    DupePlayer,
    #[error("IO error: {_0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
