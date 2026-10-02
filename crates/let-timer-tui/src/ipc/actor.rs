//! The IPC actor: the one task that talks to the daemon.
//!
//! [`IpcClient::request`] is not cancel-safe. If the event loop dropped a
//! request that was halfway through a write, the socket would be left with a
//! partial command and every later reply would be off by one. So the event
//! loop never holds the connection: it hands a command to this actor and waits
//! for the answer, and the actor is the only thing that owns the socket.

use std::path::PathBuf;
use std::time::Duration;

use let_timer_core::{Command, IpcClient, Response};
use tokio::sync::{mpsc, oneshot};

/// Shown when the actor task is gone, which means nothing will ever answer.
pub(crate) const ACTOR_GONE: &str = "the daemon connection task stopped";

/// Shown when the socket cannot be reached at all.
pub(crate) const CONNECT_FAILED: &str = "cannot reach the let-timer daemon";

/// Shown when the daemon accepted the command but never answered it.
pub(crate) const NO_REPLY: &str = "the daemon did not answer in time";

/// How long the event loop waits for a reply before reporting a failure.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// What the event loop asks the actor to do.
#[derive(Debug)]
enum Request {
    /// Send one command, then answer with the daemon's reply or an error.
    Send {
        command: Command,
        reply: oneshot::Sender<Result<Response, String>>,
    },
}

/// A handle to the actor task.
///
/// Cheap to clone and holds no socket, so the event loop can pass it around
/// freely.
#[derive(Debug, Clone)]
pub struct IpcActor {
    requests: mpsc::UnboundedSender<Request>,
    reply_timeout: Duration,
}

impl IpcActor {
    /// Start an actor that will connect to the daemon at `path`.
    ///
    /// Connecting is deferred to the first request, so spawning this never
    /// fails and never blocks.
    pub fn spawn(path: impl Into<PathBuf>) -> Self {
        Self::spawn_with_timeout(path, REPLY_TIMEOUT)
    }

    /// Start an actor that gives up on a reply after `reply_timeout`.
    pub fn spawn_with_timeout(path: impl Into<PathBuf>, reply_timeout: Duration) -> Self {
        let (requests, inbox) = mpsc::unbounded_channel();
        tokio::spawn(actor_loop(path.into(), inbox));
        Self {
            requests,
            reply_timeout,
        }
    }

    /// Send a command and wait for the reply.
    ///
    /// Errors come back as text: `IpcClient::request` returns a boxed trait
    /// object that cannot cross the channel boundary.
    ///
    /// When the wait times out the request is left to finish on its own
    /// instead of being cancelled. Cancelling it mid-read is exactly what this
    /// actor exists to prevent: the socket would be left waiting on a reply
    /// that a later request then reads, and every answer after that would be
    /// off by one. The daemon is single-threaded per connection, so a late
    /// reply simply lands in a channel nobody is listening on.
    pub async fn send(&self, command: Command) -> Result<Response, String> {
        let (reply, answer) = oneshot::channel();
        self.requests
            .send(Request::Send { command, reply })
            .map_err(|_| ACTOR_GONE.to_string())?;

        match tokio::time::timeout(self.reply_timeout, answer).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(ACTOR_GONE.to_string()),
            Err(_) => Err(NO_REPLY.to_string()),
        }
    }
}

/// The actor task: owns the connection and answers requests in order.
async fn actor_loop(path: PathBuf, mut inbox: mpsc::UnboundedReceiver<Request>) {
    let mut client: Option<IpcClient> = None;

    while let Some(request) = inbox.recv().await {
        match request {
            Request::Send { command, reply } => {
                if client.is_none() {
                    client = match IpcClient::connect(&path).await {
                        Ok(client) => Some(client),
                        Err(error) => {
                            let _ = reply.send(Err(format!("{CONNECT_FAILED}: {error}")));
                            continue;
                        }
                    };
                }

                // The connection exists, so this unwrap cannot fail.
                let connection = client.as_mut().expect("just connected");
                let outcome = match connection.request(&command).await {
                    Ok(response) => Ok(response),
                    Err(error) => {
                        // Half a request may be sitting in the socket, so this
                        // client can no longer be trusted to line replies up
                        // with commands. Throw it away and let the next request
                        // open a fresh connection.
                        client = None;
                        Err(error.to_string())
                    }
                };
                let _ = reply.send(outcome);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::test_daemon::{Behaviour, socket_path, spawn_daemon};

    fn list_command() -> Command {
        Command::List {
            sort_priority: None,
            filter_status: None,
        }
    }

    #[tokio::test]
    async fn a_command_comes_back_as_a_response() {
        let path = socket_path();
        let _daemon = spawn_daemon(path.clone(), Behaviour::Answer(Response::OkEmpty)).await;

        let actor = IpcActor::spawn(path);
        let response = actor.send(list_command()).await;

        assert!(matches!(response, Ok(Response::OkEmpty)));
    }

    #[tokio::test]
    async fn the_actor_answers_several_commands_in_order() {
        let path = socket_path();
        let _daemon = spawn_daemon(path.clone(), Behaviour::Answer(Response::OkEmpty)).await;

        let actor = IpcActor::spawn(path);
        for _ in 0..3 {
            assert!(matches!(
                actor.send(list_command()).await,
                Ok(Response::OkEmpty)
            ));
        }
    }

    #[tokio::test]
    async fn a_missing_socket_is_reported_not_hung() {
        // Nothing is listening here.
        let path = socket_path();

        let actor = IpcActor::spawn(path);
        let error = actor.send(list_command()).await.unwrap_err();

        assert!(
            error.starts_with(CONNECT_FAILED),
            "expected a connection complaint, got {error:?}"
        );
    }

    #[tokio::test]
    async fn the_actor_can_be_cloned() {
        let path = socket_path();
        let _daemon = spawn_daemon(path.clone(), Behaviour::Answer(Response::OkEmpty)).await;

        let actor = IpcActor::spawn(path);
        let copy = actor.clone();

        assert!(matches!(
            copy.send(list_command()).await,
            Ok(Response::OkEmpty)
        ));
    }
}

#[cfg(test)]
mod timeout_tests {
    use super::*;
    use crate::ipc::test_daemon::{Behaviour, socket_path, spawn_daemon};

    fn list_command() -> Command {
        Command::List {
            sort_priority: None,
            filter_status: None,
        }
    }

    #[tokio::test]
    async fn a_daemon_that_never_answers_gives_up_in_time() {
        let path = socket_path();
        let _daemon = spawn_daemon(path.clone(), Behaviour::Hang).await;

        let actor = IpcActor::spawn_with_timeout(path, Duration::from_millis(50));
        let error = actor.send(list_command()).await.unwrap_err();

        assert_eq!(error, NO_REPLY);
    }

    #[tokio::test]
    async fn the_default_timeout_is_five_seconds() {
        assert_eq!(REPLY_TIMEOUT, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn a_reply_that_arrives_too_late_is_dropped_not_misread() {
        let path = socket_path();
        let _daemon = spawn_daemon(
            path.clone(),
            Behaviour::SlowFirst {
                delay: Duration::from_millis(300),
                response: Response::OkEmpty,
            },
        )
        .await;

        let actor = IpcActor::spawn_with_timeout(path, Duration::from_millis(100));
        assert_eq!(actor.send(list_command()).await.unwrap_err(), NO_REPLY);

        // Let the abandoned request finish on the wire; its late reply lands
        // in a channel nobody is reading.
        tokio::time::sleep(Duration::from_millis(400)).await;

        // The next request must get its own answer rather than the stale one.
        let answer = actor
            .send(list_command())
            .await
            .expect("the next request should still be answered");

        assert!(matches!(answer, Response::OkEmpty));
    }

    #[tokio::test]
    async fn a_reply_inside_the_timeout_is_returned_normally() {
        let path = socket_path();
        let _daemon = spawn_daemon(
            path.clone(),
            Behaviour::SlowFirst {
                delay: Duration::from_millis(10),
                response: Response::OkEmpty,
            },
        )
        .await;

        let actor = IpcActor::spawn_with_timeout(path, Duration::from_secs(5));

        assert!(matches!(
            actor.send(list_command()).await,
            Ok(Response::OkEmpty)
        ));
    }
}

#[cfg(test)]
mod reconnect_tests {
    use super::*;
    use crate::ipc::test_daemon::{Behaviour, socket_path, spawn_daemon};
    use crate::store::test_util::{task_with_id, tasks};

    fn list_command() -> Command {
        Command::List {
            sort_priority: None,
            filter_status: None,
        }
    }

    #[tokio::test]
    async fn a_daemon_that_hangs_up_is_reported_as_an_error() {
        let path = socket_path();
        let _daemon = spawn_daemon(path.clone(), Behaviour::DropConnection).await;

        let actor = IpcActor::spawn(path);
        let error = actor.send(list_command()).await.unwrap_err();

        assert!(
            error.to_lowercase().contains("closed"),
            "expected a closed-connection complaint, got {error:?}"
        );
    }

    #[tokio::test]
    async fn the_actor_connects_later_when_the_daemon_appears() {
        // Nothing is listening yet, so the first attempt has to fail.
        let path = socket_path();
        let actor = IpcActor::spawn(path.clone());

        assert!(actor.send(list_command()).await.is_err());

        // The daemon starts up; the next request should find it.
        let _daemon = spawn_daemon(path, Behaviour::Answer(Response::OkEmpty)).await;

        assert!(matches!(
            actor.send(list_command()).await,
            Ok(Response::OkEmpty)
        ));
    }

    #[tokio::test]
    async fn a_dropped_connection_does_not_poison_later_requests() {
        // Two daemons on the same path, in turn: the first hangs up, the second
        // answers properly.
        let path = socket_path();
        let actor = IpcActor::spawn(path.clone());

        let first = spawn_daemon(path.clone(), Behaviour::DropConnection).await;
        assert!(actor.send(list_command()).await.is_err());
        drop(first);

        let second = spawn_daemon(path, Behaviour::Answer(Response::OkList(tasks(2)))).await;
        let answer = actor.send(list_command()).await;

        drop(second);
        match answer {
            Ok(Response::OkList(list)) => assert_eq!(list.len(), 2),
            other => panic!("expected a fresh list, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_reply_after_a_reconnect_is_a_full_list() {
        let path = socket_path();
        let actor = IpcActor::spawn(path.clone());

        let first = spawn_daemon(path.clone(), Behaviour::DropConnection).await;
        assert!(actor.send(list_command()).await.is_err());
        drop(first);

        let _second = spawn_daemon(
            path,
            Behaviour::Answer(Response::OkList(vec![task_with_id(1)])),
        )
        .await;

        match actor.send(list_command()).await {
            Ok(Response::OkList(list)) => assert_eq!(list.len(), 1),
            other => panic!("expected a list, got {other:?}"),
        }
    }
}
