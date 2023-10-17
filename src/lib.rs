// Using nightly for async traits and unwarp or clone
#![feature(arc_unwrap_or_clone)]
#![feature(async_fn_in_trait)]
mod app;
mod appevent;
mod core;

use tokio::runtime;

// Add tests

// XXX Should err
pub fn run_app() -> Result<(), std::io::Error> {
    let rt = runtime::Runtime::new()?;
    rt.block_on(async {
        // TODO: Handle errors
        let mut app = app::Youtui::new().unwrap();
        app.run().await;
    });
    Ok(())
}

pub mod error {
    use std::fmt::Display;

    use tokio::sync::mpsc;

    pub type Result<T> = std::result::Result<T, Error>;
    #[derive(Debug, Clone)]
    pub enum Error {
        Communication,
    }
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Error sending message to channel")
        }
    }
    impl<T> From<mpsc::error::SendError<T>> for Error {
        fn from(_value: mpsc::error::SendError<T>) -> Self {
            Error::Communication
        }
    }
}
