pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // TODO: Investigate if this is a valid branch.
    ErrorSending,
    ReceiverDropped,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorSending => write!(f, "Error sending to a channel."),
            Error::ReceiverDropped => write!(f, "Error sending to a channel, receiver dropped."),
        }
    }
}
