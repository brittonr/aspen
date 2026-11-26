//! The distinct listener peer process: bind, hand off the endpoint, serve one
//! session and report what it observed.

/// Listener peer: bind, hand off the endpoint, serve one session, report.
pub(super) async fn serve(run_directory: &std::path::Path, should_withhold: bool) -> crate::error::Result<()> {
    let mut listener = crate::fabric_transport::Listener::bind(crate::fabric_transport::ListenerInput {
        profile: super::fixture::profile()?,
        protocol: super::fixture::protocol(),
        capability: crate::fabric_transport::EndpointCapability::from_secret_bytes(
            super::LISTENER_SECRET,
            super::fixture::reference("qualification listener capability"),
        )?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        listener_identity_ref: super::fixture::reference("qualification listener identity"),
        expected_peer_context_ref: super::fixture::reference("qualification expected peer context"),
        locator_cohort_ref: super::fixture::reference("qualification locator cohort"),
        disclosure: super::fixture::disclosure(),
        validity: super::fixture::validity(),
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        observed_tick: super::OBSERVED_TICK,
        // Exactly the producer's fixed client endpoint is admitted, under this
        // listener's own peer context; any other authenticated endpoint is refused.
        admitted_peers: std::collections::BTreeMap::from([(
            super::CLIENT_PEER_ID.to_string(),
            crate::fabric_transport::AdmittedPeer {
                public_endpoint_identity: super::fixture::public_identity(super::CLIENT_SECRET),
                peer_context_ref: super::fixture::reference("qualification expected peer context"),
            },
        )]),
    })
    .await?;
    super::super::peer::write_value(&run_directory.join(super::ENDPOINT_FILE), &listener.handoff().value)?;
    let (outcome, frame, payload_ref) = accept(&mut listener, should_withhold).await?;
    // Any connection inside the window, admitted or refused, would be a retry.
    let second = listener
        .poll_one_derived_frame(&super::fixture::session_ref(), super::RETRY_WINDOW, |_peer_id, _payload| {
            Ok(super::fixture::request_ref())
        })
        .await?;
    let cleanup = listener.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
    super::super::peer::write_value(
        &run_directory.join(super::LISTENER_REPORT_FILE),
        &crate::preserves_rail::record("qualification-transport-listener-v1", vec![
            crate::preserves_rail::string(outcome),
            crate::preserves_rail::string(&payload_ref),
            frame
                .as_ref()
                .map_or_else(|| crate::preserves_rail::sequence(Vec::new()), super::fixture::frame_projection),
            crate::preserves_rail::bool_value(!matches!(second, crate::fabric_transport::ListenerPoll::Idle)),
            crate::preserves_rail::string(format!("{:?}", cleanup.terminal_class)),
        ]),
    )
}

/// Serve the one session: its outcome label, delivered frame evidence and the
/// payload ref the listener read.
async fn accept(
    listener: &mut crate::fabric_transport::Listener,
    should_withhold: bool,
) -> crate::error::Result<(&'static str, Option<crate::fabric_transport::FrameEvidence>, String)> {
    if !should_withhold {
        let received = listener
            .accept_one_frame(&super::fixture::session_ref(), &super::fixture::request_ref(), super::EXCHANGE_TIMEOUT)
            .await?;
        let payload_ref = received.evidence.payload_ref.clone();
        return Ok(("delivered", Some(received.evidence), payload_ref));
    }
    // The frame is read, then the acknowledgement is withheld: the
    // submission outcome is unknown to the client. Only a frame actually
    // read inside the derivation counts as a withheld acknowledgement; an
    // accept timeout or transport error is a distinct outcome.
    let mut withheld_payload_ref = None;
    let refused = listener
        .accept_one_derived_frame(&super::fixture::session_ref(), super::EXCHANGE_TIMEOUT, |payload| {
            withheld_payload_ref = Some(crate::fabric_transport::frame_ref(&super::fixture::request_ref(), payload));
            Err(super::super::super::invalid("qualification listener withholds the acknowledgement"))
        })
        .await;
    Ok(match (refused, withheld_payload_ref) {
        (Err(_), Some(payload_ref)) => ("acknowledgement-withheld", None, payload_ref),
        (Err(_), None) => ("no-frame-received", None, String::new()),
        (Ok(received), _) => {
            let payload_ref = received.evidence.payload_ref.clone();
            ("delivered", Some(received.evidence), payload_ref)
        }
    })
}
