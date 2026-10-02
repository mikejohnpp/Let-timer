//! Terminal UI for let-timer.
//!
//! Built on the Flux architecture: every change to the application state is an
//! `Action`, dispatched through a `Dispatcher` to the stores that care about
//! it. Stores return `Effect`s for the outside world to carry out, and views
//! only ever read a read-only `Snapshot` of the state.

pub mod action;
pub mod config;
pub mod dispatcher;
pub mod effect;
pub mod ipc;
pub mod store;
