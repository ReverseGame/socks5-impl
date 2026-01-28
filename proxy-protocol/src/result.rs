pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid signature, expected PROXY protocol v2")]
    InvalidSignature,

    #[error("unsupported version: {0}, expected 2")]
    UnsupportedVersion(u8),

    #[error("invalid command: {0}")]
    InvalidCommand(u8),

    #[error("invalid address family: {0:#x}")]
    InvalidAddressFamily(u8),

    #[error("frame too short: got {got} bytes, need at least {need}")]
    FrameTooShort { got: usize, need: usize },

    #[error("address data length mismatch for {family}: got {got}, expected {expected}")]
    AddressLengthMismatch { family: &'static str, got: usize, expected: usize },

    #[error("invalid address length field: {0}")]
    InvalidAddressLength(u16),

    // 向后兼容的通用错误
    #[error("{0}")]
    String(String),

    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::String(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::String(s)
    }
}
