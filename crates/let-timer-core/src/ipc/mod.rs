//! Talking to the daemon over a Unix socket.
//!
//! Both ends agree on one path, worked out by [`socket_path`], so that neither
//! side has to be told where the other is.

pub mod client;
pub mod server;

use std::path::PathBuf;

/// Where the daemon listens, and where clients go looking for it.
///
/// The per-user runtime directory is preferred, because nothing else can write
/// there. A socket left in `/tmp` is a name anybody can take first, and a
/// program that trusts a name like that hands its commands — and with them the
/// path of the database — to whoever got there first.
///
/// `$XDG_RUNTIME_DIR` is not always set, and a daemon with nowhere to listen
/// would be no use at all, so there is a fallback. It is a poor one, and a
/// multi-user machine is the place where it bites.
pub fn socket_path() -> PathBuf {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR")
        && !runtime_dir.is_empty()
    {
        return PathBuf::from(runtime_dir).join("let-timer.sock");
    }
    PathBuf::from("/tmp/let-timer.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_is_a_socket() {
        assert_eq!(
            socket_path()
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("sock"),
            "a client and a server that disagree about this cannot talk"
        );
    }
}
