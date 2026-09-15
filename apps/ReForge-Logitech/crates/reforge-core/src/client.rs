use crate::{paths, RpcRequest, RpcResponse};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
};

pub fn call(request: &RpcRequest) -> Result<RpcResponse, String> {
    call_at(&paths::runtime_socket(), request)
}

pub fn call_at(path: &std::path::Path, request: &RpcRequest) -> Result<RpcResponse, String> {
    let mut stream = UnixStream::connect(path)
        .map_err(|error| format!("cannot connect to daemon at {}: {error}", path.display()))?;
    let payload = serde_json::to_string(request)
        .map_err(|error| format!("failed to encode RPC request: {error}"))?;
    stream
        .write_all(payload.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("failed to send RPC request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush RPC request: {error}"))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("failed to read RPC response: {error}"))?;
    if line.trim().is_empty() {
        return Err("daemon closed the connection without a response".into());
    }
    serde_json::from_str(line.trim_end())
        .map_err(|error| format!("failed to decode daemon response: {error}"))
}
