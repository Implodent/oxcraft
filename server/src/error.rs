use oxcr_protocol::error::Error as NetError;
use oxcr_protocol::miette;

#[derive(oxcr_protocol::thiserror::Error, miette::Diagnostic, Debug)]
pub enum Error {
    #[error("{_0}")]
    #[diagnostic(transparent)]
    Net(NetError),
    #[error("Incorrect protocol version: {_0}")]
    #[diagnostic(code(server::error::incorrect_protocol_version))]
    IncorrectVersion(i32), // #[error("duplicate player IP")]
                           // DupePlayer,
}

impl<T: Into<NetError>> From<T> for Error {
    fn from(value: T) -> Self {
        Self::Net(value.into())
    }
}
