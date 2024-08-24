use super::{api, downloader, player};
use crate::app::taskmanager::{KillableTask, TaskID};

#[derive(Debug)]
pub struct KillRequest;

// Server request MUST be an enum, whilst it's tempting to use structs here to
// take advantage of generics, every message sent to channel must be the same
// size.
#[derive(Debug)]
pub enum ServerRequest {
    Killable {
        killable_task: KillableTask,
        request: KillableServerRequest,
    },
    Unkillable {
        task_id: TaskID,
        request: UnkillableServerRequest,
    },
}

#[derive(Debug)]
pub enum KillableServerRequest {
    Api(api::KillableServerRequest),
    Player(player::KillableServerRequest),
    Downloader(downloader::KillableServerRequest),
}

// Whilst not all fields are currently used, they're a key component of the
// architecture and expected to be used in future.
#[allow(unused)]
#[derive(Debug)]
pub enum UnkillableServerRequest {
    Api(api::UnkillableServerRequest),
    Player(player::UnkillableServerRequest),
    Downloader(downloader::UnkillableServerRequest),
}

#[derive(Debug)]
pub struct ServerResponse {
    pub id: TaskID,
    pub response: ServerResponseType,
}

impl ServerResponse {
    pub fn new_api(id: TaskID, response: api::Response) -> Self {
        Self {
            id,
            response: ServerResponseType::Api(response),
        }
    }
    pub fn new_player(id: TaskID, response: player::Response) -> Self {
        Self {
            id,
            response: ServerResponseType::Player(response),
        }
    }
    pub fn new_downloader(id: TaskID, response: downloader::Response) -> Self {
        Self {
            id,
            response: ServerResponseType::Downloader(response),
        }
    }
}

#[derive(Debug)]
pub enum ServerResponseType {
    Api(api::Response),
    Player(player::Response),
    Downloader(downloader::Response),
}
