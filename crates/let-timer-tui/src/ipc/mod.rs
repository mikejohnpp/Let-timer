//! Talking to the daemon.

pub mod actor;

#[cfg(test)]
pub(crate) mod test_daemon;

pub use actor::IpcActor;
