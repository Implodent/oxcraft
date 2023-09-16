use oxcr_protocol::error::Error as NetError;

#[derive(oxcr_protocol::thiserror::Error, Debug)]
pub enum Error {
    #[error("{_0}")]
    Net(NetError),
    // #[error("duplicate player IP")]
    // DupePlayer,
}

impl<T: Into<NetError>> From<T> for Error {
    fn from(value: T) -> Self {
        Self::Net(value.into())
    }
}
