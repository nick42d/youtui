use tokio::sync::mpsc::{self, error::TryRecvError};

pub enum TryRecvManyOutcome<T> {
    Finished(Vec<T>),
    NotFinished(Vec<T>),
}

pub fn mpsc_try_recv_many<T>(receiver: &mut mpsc::Receiver<T>) -> TryRecvManyOutcome<T> {
    let mut buf = vec![];
    loop {
        match receiver.try_recv() {
            Ok(item) => buf.push(item),
            Err(TryRecvError::Empty) => return TryRecvManyOutcome::NotFinished(buf),
            Err(TryRecvError::Disconnected) => return TryRecvManyOutcome::Finished(buf),
        }
    }
}
