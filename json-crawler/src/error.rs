#[derive(Debug)]
pub struct Error;

pub type Result<T> = std::result::Result<T, Error>

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}
