pub mod ipc;
pub mod protocol;

pub use ipc::client::IpcClient;
pub use ipc::server::IPCServer;
pub use protocol::{Command, Response};