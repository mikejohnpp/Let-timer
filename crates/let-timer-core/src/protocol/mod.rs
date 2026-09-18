use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Command {
    Start,
    Stop,
    End,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Response {
    pub data: String,
}
