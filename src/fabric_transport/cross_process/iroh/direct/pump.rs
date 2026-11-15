use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use super::*;

const DATA: u8 = 1;
const ACK: u8 = 2;
const FIN: u8 = 3;
const HEADER_BYTES: usize = 9;

pub(super) struct Context<'a, S> {
    pub(super) service: &'a mut S,
    pub(super) admitted: &'a AdmittedOperation,
    pub(super) bounds: &'a DirectStreamBounds,
    pub(super) progress: &'a mut Progress,
    pub(super) session: &'a mut CrossProcessSessionState,
}

struct Transfer {
    outbound: Vec<u8>,
    inbound: Vec<u8>,
    header: [u8; HEADER_BYTES],
    header_bytes: usize,
    next_sequence: u64,
    receive_sequence: u64,
    pending: Option<(u64, u64, blake3::Hash)>,
    empty_digest: blake3::Hash,
    is_local_eof: bool,
    is_remote_eof: bool,
    idle: std::time::Duration,
}

/// One stream, at most one unacknowledged frame in each direction. The receiver
/// acknowledges only after its bounded local write completes. Neither the ACK
/// nor socket write claims durability or application completion.
pub(super) async fn run<S: ProtectedSocketService>(
    send: &mut iroh::endpoint::SendStream,
    receive: &mut iroh::endpoint::RecvStream,
    socket: tokio::net::UnixStream,
    mut context: Context<'_, S>,
) -> Result<(), DirectStreamTerminal> {
    let capacity = usize::try_from(context.bounds.max_frame_bytes).map_err(|_| DirectStreamTerminal::OverBound)?;
    let mut transfer = Transfer {
        outbound: vec![0_u8; capacity],
        inbound: vec![0_u8; capacity],
        header: [0_u8; HEADER_BYTES],
        header_bytes: 0,
        next_sequence: 1,
        receive_sequence: 1,
        pending: None,
        empty_digest: blake3::hash(&[]),
        is_local_eof: false,
        is_remote_eof: false,
        idle: std::time::Duration::from_millis(context.bounds.idle_timeout_ms),
    };
    let (mut local_read, mut local_write) = socket.into_split();
    loop {
        if transfer.is_local_eof && transfer.is_remote_eof && transfer.pending.is_none() {
            break;
        }
        context.service.check_current(context.admitted).map_err(|_| DirectStreamTerminal::Denied)?;
        // Reserve only a one-byte overflow probe after an exact bound. Consumed
        // probes remain visible as uncertainty and are never replayed.
        let next_read = transfer.read_capacity(&context)?;
        tokio::select! {
            read = tokio::time::timeout(transfer.idle, local_read.read(&mut transfer.outbound[..next_read])),
                if !transfer.is_local_eof && transfer.pending.is_none() => {
                let count = read.map_err(|_| DirectStreamTerminal::Timeout)?
                    .map_err(|_| DirectStreamTerminal::Disconnected)?;
                transfer.send_local(count, send, &mut context).await?;
            }
            // read(), unlike read_exact(), is cancellation safe. Preserve a
            // partial header across local readiness rather than replaying it.
            read = tokio::time::timeout(transfer.idle, receive.read(&mut transfer.header[transfer.header_bytes..])) => {
                let count = read.map_err(|_| DirectStreamTerminal::Timeout)?
                    .map_err(read_error)?
                    .ok_or(DirectStreamTerminal::Disconnected)?;
                if count == 0 { return Err(DirectStreamTerminal::Disconnected); }
                transfer.header_bytes += count;
                if transfer.header_bytes != HEADER_BYTES { continue; }
                transfer.header_bytes = 0;
                transfer.receive_header(send, receive, &mut local_write, &mut context).await?;
            }
        }
    }
    send.finish().map_err(|_| DirectStreamTerminal::Disconnected)?;
    let finish = async {
        // A connection close may discard still-buffered stream bytes. Observe
        // the peer's EOF and its receipt of our final ACK/FIN before cleanup.
        let mut trailing = [0_u8; 1];
        if receive.read(&mut trailing).await.map_err(read_error)?.is_some() {
            return Err(DirectStreamTerminal::Malformed);
        }
        match send.stopped().await {
            Ok(None) => Ok(()),
            Ok(Some(_)) => Err(DirectStreamTerminal::Disconnected),
            Err(error) => Err(write_error(iroh::endpoint::WriteError::from(error))),
        }
    };
    tokio::time::timeout(transfer.idle, finish).await.map_err(|_| DirectStreamTerminal::Timeout)?
}

impl Transfer {
    fn read_capacity<S>(&self, context: &Context<'_, S>) -> Result<usize, DirectStreamTerminal> {
        if context.progress.sent_frames >= context.bounds.max_frames {
            return Ok(1);
        }
        let remaining = context.bounds.max_total_bytes.saturating_sub(context.progress.sent_bytes);
        // Clamp in the wire-width domain before conversion; no platform-sized
        // sentinel can silently turn a failed conversion into an admitted read.
        let capacity = u64::try_from(self.outbound.len()).map_err(|_| DirectStreamTerminal::OverBound)?;
        let probe = remaining.saturating_add(1).min(capacity);
        usize::try_from(probe).map_err(|_| DirectStreamTerminal::OverBound)
    }

    async fn send_local<S: ProtectedSocketService>(
        &mut self,
        count: usize,
        send: &mut iroh::endpoint::SendStream,
        context: &mut Context<'_, S>,
    ) -> Result<(), DirectStreamTerminal> {
        let bytes = u64::try_from(count).map_err(|_| DirectStreamTerminal::OverBound)?;
        context.progress.local_consumed_bytes =
            context.progress.local_consumed_bytes.checked_add(bytes).ok_or(DirectStreamTerminal::OverBound)?;
        context.service.check_current(context.admitted).map_err(|_| DirectStreamTerminal::Denied)?;
        if count == 0 {
            write_header(send, FIN, 0, self.idle).await?;
            self.pending = Some((self.next_sequence, 0, self.empty_digest));
            self.next_sequence += 1;
            self.is_local_eof = true;
            return Ok(());
        }
        if context.progress.sent_frames >= context.bounds.max_frames
            || context.progress.sent_bytes.checked_add(bytes).is_none_or(|n| n > context.bounds.max_total_bytes)
        {
            return Err(DirectStreamTerminal::OverBound);
        }
        let digest = blake3::hash(&self.outbound[..count]);
        // Submission is conservative: a partial write is never definitive non-delivery.
        apply_session(context.session, CrossProcessSessionCommand::QueueFrame {
            payload_bytes: bytes,
            callback_generation: context.admitted.request.generation,
        })?;
        apply_session(context.session, CrossProcessSessionCommand::FrameSubmitted {
            payload_bytes: bytes,
            callback_generation: context.admitted.request.generation,
        })?;
        context.progress.submitted = true;
        write_header(send, DATA, bytes, self.idle).await?;
        tokio::time::timeout(self.idle, send.write_all(digest.as_bytes()))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(write_error)?;
        tokio::time::timeout(self.idle, send.write_all(&self.outbound[..count]))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(write_error)?;
        context.progress.sent_bytes += bytes;
        context.progress.sent_frames += 1;
        self.pending = Some((self.next_sequence, bytes, digest));
        self.next_sequence += 1;
        Ok(())
    }

    async fn receive_header<S: ProtectedSocketService>(
        &mut self,
        send: &mut iroh::endpoint::SendStream,
        receive: &mut iroh::endpoint::RecvStream,
        local_write: &mut tokio::net::unix::OwnedWriteHalf,
        context: &mut Context<'_, S>,
    ) -> Result<(), DirectStreamTerminal> {
        match self.header[0] {
            DATA => self.receive_data(send, receive, local_write, context).await,
            ACK => {
                let argument =
                    u64::from_be_bytes(self.header[1..].try_into().map_err(|_| DirectStreamTerminal::Malformed)?);
                self.receive_ack(argument, receive, context).await
            }
            FIN => {
                let argument =
                    u64::from_be_bytes(self.header[1..].try_into().map_err(|_| DirectStreamTerminal::Malformed)?);
                if argument != 0 || self.is_remote_eof {
                    return Err(DirectStreamTerminal::Malformed);
                }
                context.service.check_current(context.admitted).map_err(|_| DirectStreamTerminal::Denied)?;
                tokio::time::timeout(self.idle, local_write.shutdown())
                    .await
                    .map_err(|_| DirectStreamTerminal::Timeout)?
                    .map_err(|_| DirectStreamTerminal::Disconnected)?;
                self.is_remote_eof = true;
                write_header(send, ACK, self.receive_sequence, self.idle).await?;
                tokio::time::timeout(self.idle, send.write_all(self.empty_digest.as_bytes()))
                    .await
                    .map_err(|_| DirectStreamTerminal::Timeout)?
                    .map_err(write_error)?;
                self.receive_sequence += 1;
                Ok(())
            }
            _ => Err(DirectStreamTerminal::Malformed),
        }
    }

    async fn receive_data<S: ProtectedSocketService>(
        &mut self,
        send: &mut iroh::endpoint::SendStream,
        receive: &mut iroh::endpoint::RecvStream,
        local_write: &mut tokio::net::unix::OwnedWriteHalf,
        context: &mut Context<'_, S>,
    ) -> Result<(), DirectStreamTerminal> {
        let argument = u64::from_be_bytes(self.header[1..].try_into().map_err(|_| DirectStreamTerminal::Malformed)?);
        if self.is_remote_eof
            || argument == 0
            || argument > context.bounds.max_frame_bytes
            || context.progress.received_frames >= context.bounds.max_frames
            || context
                .progress
                .received_bytes
                .checked_add(argument)
                .is_none_or(|n| n > context.bounds.max_total_bytes)
        {
            return Err(DirectStreamTerminal::OverBound);
        }
        let count = usize::try_from(argument).map_err(|_| DirectStreamTerminal::OverBound)?;
        let mut digest = [0_u8; 32];
        tokio::time::timeout(self.idle, receive.read_exact(&mut digest))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(exact_read_error)?;
        tokio::time::timeout(self.idle, receive.read_exact(&mut self.inbound[..count]))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(exact_read_error)?;
        if blake3::hash(&self.inbound[..count]).as_bytes() != &digest {
            return Err(DirectStreamTerminal::Malformed);
        }
        context.service.check_current(context.admitted).map_err(|_| DirectStreamTerminal::Denied)?;
        // Mark uncertainty before a potentially partial protected local write.
        apply_session(context.session, CrossProcessSessionCommand::ReceiveFrame {
            payload_bytes: argument,
            callback_generation: context.admitted.request.generation,
        })?;
        context.progress.received_bytes += argument;
        context.progress.received_frames += 1;
        tokio::time::timeout(self.idle, local_write.write_all(&self.inbound[..count]))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(|_| DirectStreamTerminal::Disconnected)?;
        write_header(send, ACK, self.receive_sequence, self.idle).await?;
        tokio::time::timeout(self.idle, send.write_all(&digest))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(write_error)?;
        self.receive_sequence += 1;
        Ok(())
    }

    async fn receive_ack<S>(
        &mut self,
        argument: u64,
        receive: &mut iroh::endpoint::RecvStream,
        context: &mut Context<'_, S>,
    ) -> Result<(), DirectStreamTerminal> {
        let (expected, bytes, digest) = self.pending.ok_or(DirectStreamTerminal::Malformed)?;
        if argument != expected {
            return Err(DirectStreamTerminal::Malformed);
        }
        let mut acknowledged = [0_u8; 32];
        tokio::time::timeout(self.idle, receive.read_exact(&mut acknowledged))
            .await
            .map_err(|_| DirectStreamTerminal::Timeout)?
            .map_err(exact_read_error)?;
        if digest.as_bytes() != &acknowledged {
            return Err(DirectStreamTerminal::Malformed);
        }
        if bytes != 0 {
            apply_session(context.session, CrossProcessSessionCommand::AcknowledgeFrame {
                payload_bytes: bytes,
                callback_generation: context.admitted.request.generation,
            })?;
        }
        context.progress.acknowledged_bytes += bytes;
        self.pending = None;
        Ok(())
    }
}

include!("pump/parts/wire/body.rs");
include!("pump/parts/lifecycle/body.rs");
