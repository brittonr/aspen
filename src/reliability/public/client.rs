//! Bounded public TCP adapter. Only public responses enter model histories.

// r[impl molten.consensus.live_public_adapter]
pub fn execute(request: &super::protocol::Request) -> super::protocol::Response {
    match exchange(request, None) {
        Ok(response) => response,
        Err(diagnostic) => super::protocol::uncertain(request, diagnostic),
    }
}

pub fn execute_at(request: &super::protocol::Request, target: std::net::SocketAddr) -> super::protocol::Response {
    if !request.profile.as_ref().is_some_and(|profile| profile.endpoints.contains(&target.to_string())) {
        return super::protocol::uncertain(request, "forward-target-outside-admitted-cohort".into());
    }
    match exchange(request, Some(target)) {
        Ok(response) => response,
        Err(diagnostic) => super::protocol::uncertain(request, diagnostic),
    }
}

fn exchange(
    request: &super::protocol::Request,
    target: Option<std::net::SocketAddr>,
) -> Result<super::protocol::Response, String> {
    let endpoint = target.unwrap_or(super::protocol::validate_request(request)?);
    let timeout = std::time::Duration::from_millis(request.deadline_ms);
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(timeout)
        .map_err(|error| format!("deadline-uncertain:{error}"))?;
    let mut stream = std::net::TcpStream::connect_timeout(&endpoint, remaining(&mut deadline)?)
        .map_err(|error| format!("connect-uncertain:{error}"))?;
    stream.set_write_timeout(Some(remaining(&mut deadline)?)).map_err(|error| error.to_string())?;
    let mut bytes = serde_json::to_vec(request).map_err(|error| error.to_string())?;
    if bytes.len() as u64 >= super::protocol::MAX_FRAME_BYTES {
        return Err("request-frame-bound".into());
    }
    bytes.push(b'\n');
    std::io::Write::write_all(&mut stream, &bytes).map_err(|error| format!("write-uncertain:{error}"))?;
    let bytes = read_frame(&mut stream, &mut deadline)?;
    let response = serde_json::from_slice(&bytes).map_err(|error| format!("malformed-response:{error}"))?;
    super::protocol::validate_response(request, response)
}

fn remaining(deadline: &mut crate::fabric_time::SupervisionDeadline) -> Result<std::time::Duration, String> {
    let remaining = deadline.remaining().map_err(|error| format!("deadline-uncertain:{error}"))?;
    if remaining.is_zero() {
        return Err("deadline-uncertain".into());
    }
    Ok(remaining)
}

fn read_frame(
    stream: &mut std::net::TcpStream,
    deadline: &mut crate::fabric_time::SupervisionDeadline,
) -> Result<Vec<u8>, String> {
    let mut reader = std::io::BufReader::new(stream);
    let mut bytes = Vec::new();
    loop {
        reader.get_mut().set_read_timeout(Some(remaining(deadline)?)).map_err(|error| error.to_string())?;
        let available = std::io::BufRead::fill_buf(&mut reader).map_err(|error| format!("read-uncertain:{error}"))?;
        if available.is_empty() {
            return Err("disconnect-uncertain".into());
        }
        let count = available.iter().position(|byte| *byte == b'\n').map_or(available.len(), |position| position + 1);
        if (bytes.len() + count) as u64 > super::protocol::MAX_FRAME_BYTES {
            return Err("response-frame-bound".into());
        }
        let is_complete = available[count - 1] == b'\n';
        bytes.extend_from_slice(&available[..count]);
        std::io::BufRead::consume(&mut reader, count);
        if is_complete {
            return Ok(bytes);
        }
    }
}
