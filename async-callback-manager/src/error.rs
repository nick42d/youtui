pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ErrorSending,
    ReceiverDropped,
}
