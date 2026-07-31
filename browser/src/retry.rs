use std::io::ErrorKind;

/// True when a failed CDP command is a dropped or stalled transport rather than
/// a real protocol error, so the command is worth retrying on a fresh socket.
pub fn reconnectable(error: &anyhow::Error) -> bool {
    if transport_lost(error) {
        return true;
    }
    let message = format!("{error:#}");
    message.contains("timed out") || message.contains("disconnected")
}

/// True when the browser connection itself is gone. Unlike a stalled command,
/// this means no further evidence can be collected, so callers must not treat
/// the remaining work as merely skipped.
pub fn transport_lost(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .any(|cause| dropped_connection(cause.kind()))
}

fn dropped_connection(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::BrokenPipe
            | ErrorKind::UnexpectedEof
            | ErrorKind::NotConnected
    )
}

#[cfg(test)]
mod tests {
    use super::reconnectable;
    use std::io::{Error, ErrorKind};

    fn wrapped(kind: ErrorKind, message: &str) -> anyhow::Error {
        let io = Error::new(kind, message.to_string());
        let websocket = tokio_tungstenite::tungstenite::Error::Io(io);
        anyhow::Error::new(websocket).context("Runtime.evaluate")
    }

    #[test]
    fn windows_forcibly_closed_connection_is_retried() {
        let error = wrapped(
            ErrorKind::ConnectionReset,
            "An existing connection was forcibly closed by the remote host. (os error 10054)",
        );
        assert!(reconnectable(&error));
    }

    #[test]
    fn every_dropped_transport_kind_is_retried() {
        for kind in [
            ErrorKind::ConnectionReset,
            ErrorKind::ConnectionAborted,
            ErrorKind::BrokenPipe,
            ErrorKind::UnexpectedEof,
            ErrorKind::NotConnected,
        ] {
            assert!(reconnectable(&wrapped(kind, "dropped")), "{kind:?}");
        }
    }

    #[test]
    fn stalled_and_disconnected_commands_are_retried() {
        assert!(reconnectable(&anyhow::anyhow!(
            "CDP command timed out: Runtime.evaluate"
        )));
        assert!(reconnectable(&anyhow::anyhow!("CDP disconnected")));
    }

    #[test]
    fn a_lost_transport_is_distinguished_from_a_stalled_command() {
        use super::transport_lost;
        assert!(transport_lost(&wrapped(ErrorKind::ConnectionReset, "gone")));
        assert!(!transport_lost(&anyhow::anyhow!(
            "CDP command timed out: Runtime.evaluate"
        )));
    }

    #[test]
    fn a_real_protocol_error_is_not_retried() {
        let error = anyhow::anyhow!("Runtime.evaluate: invalid parameters");
        assert!(!reconnectable(&error));
        assert!(!reconnectable(&wrapped(
            ErrorKind::PermissionDenied,
            "refused"
        )));
    }
}
