async fn write_header(
    send: &mut iroh::endpoint::SendStream,
    kind: u8,
    argument: u64,
    idle: std::time::Duration,
) -> Result<(), DirectStreamTerminal> {
    let mut bytes = [0_u8; HEADER_BYTES];
    bytes[0] = kind;
    bytes[1..].copy_from_slice(&argument.to_be_bytes());
    tokio::time::timeout(idle, send.write_all(&bytes))
        .await
        .map_err(|_| DirectStreamTerminal::Timeout)?
        .map_err(write_error)
}

pub(super) fn connection_error(error: iroh::endpoint::ConnectionError) -> DirectStreamTerminal {
    if matches!(error, iroh::endpoint::ConnectionError::TimedOut) {
        DirectStreamTerminal::Timeout
    } else {
        DirectStreamTerminal::Disconnected
    }
}

pub(super) fn read_error(error: iroh::endpoint::ReadError) -> DirectStreamTerminal {
    match error {
        iroh::endpoint::ReadError::ConnectionLost(error) => connection_error(error),
        iroh::endpoint::ReadError::Reset(_)
        | iroh::endpoint::ReadError::ClosedStream
        | iroh::endpoint::ReadError::ZeroRttRejected => DirectStreamTerminal::Disconnected,
    }
}

pub(super) fn exact_read_error(error: iroh::endpoint::ReadExactError) -> DirectStreamTerminal {
    match error {
        iroh::endpoint::ReadExactError::ReadError(error) => read_error(error),
        iroh::endpoint::ReadExactError::FinishedEarly(_) => DirectStreamTerminal::Disconnected,
    }
}

pub(super) fn write_error(error: iroh::endpoint::WriteError) -> DirectStreamTerminal {
    match error {
        iroh::endpoint::WriteError::ConnectionLost(error) => connection_error(error),
        iroh::endpoint::WriteError::Stopped(_)
        | iroh::endpoint::WriteError::ClosedStream
        | iroh::endpoint::WriteError::ZeroRttRejected => DirectStreamTerminal::Disconnected,
    }
}
