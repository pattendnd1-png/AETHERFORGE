pub mod catalog;
pub mod download;
pub mod embeddings;
pub mod registry;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use aether_core::{ProcessingPreset, Role};
use aether_model_api::{
    GenerationChunk, GenerationRequest, GenerationStream, ModelError, Tokenization,
};
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalEndpoint {
    pub port: u16,
}

impl LocalEndpoint {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
    pub fn socket_addr(self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.port)
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("local inference connection failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("local inference returned HTTP status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("local inference response was malformed: {0}")]
    Protocol(String),
    #[error("runtime process failed to start: {0}")]
    Spawn(String),
}

#[derive(Debug, Clone)]
pub struct OpenAiLocalClient {
    endpoint: LocalEndpoint,
    timeout: Duration,
}

impl OpenAiLocalClient {
    pub fn new(endpoint: LocalEndpoint) -> Self {
        Self {
            endpoint,
            timeout: Duration::from_secs(180),
        }
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn is_reachable(&self) -> bool {
        TcpStream::connect_timeout(&self.endpoint.socket_addr(), Duration::from_millis(250)).is_ok()
    }

    pub fn is_ready(&self) -> bool {
        matches!(probe_health(self.endpoint), Ok(response) if response.status == 200)
    }

    pub fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationStream, ModelError> {
        let body = request_json(&request).map_err(|e| ModelError::Provider(e.to_string()))?;
        let endpoint = self.endpoint;
        let timeout = self.timeout;
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("aetherai-model-stream".into())
            .spawn(move || {
                if let Err(error) = run_stream_request(endpoint, timeout, &body, &tx) {
                    let _ = tx.send(Err(ModelError::Provider(error.to_string())));
                }
            })
            .map_err(|e| ModelError::Provider(format!("stream worker: {e}")))?;
        Ok(rx)
    }

    pub fn tokenize(&self, input: &str) -> Result<Tokenization, ModelError> {
        let body = serde_json::to_string(&json!({
            "content": input,
            "add_special": false,
            "parse_special": true,
            "with_pieces": false
        }))
        .map_err(|error| ModelError::Provider(format!("tokenize request JSON: {error}")))?;

        run_tokenize_request(self.endpoint, self.timeout, &body)
            .map_err(|error| ModelError::Provider(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HealthResponse {
    status: u16,
    body: String,
}

fn probe_health(endpoint: LocalEndpoint) -> Result<HealthResponse, RuntimeError> {
    let mut stream =
        TcpStream::connect_timeout(&endpoint.socket_addr(), Duration::from_millis(500))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = format!(
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        endpoint.port
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| RuntimeError::Protocol(status_line.trim().into()))?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
    }
    let mut body = String::new();
    reader.read_to_string(&mut body)?;
    Ok(HealthResponse {
        status,
        body: body.trim().to_string(),
    })
}

fn request_json(request: &GenerationRequest) -> Result<String, serde_json::Error> {
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::Developer => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            json!({"role": role, "content": m.content})
        })
        .collect();
    let enable_thinking = matches!(
        request.settings.preset,
        ProcessingPreset::Deep | ProcessingPreset::Maximum
    );
    serde_json::to_string(&json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
        "temperature": request.settings.temperature,
        "top_p": request.settings.top_p,
        "max_tokens": request.settings.max_output_tokens,
        "seed": request.settings.seed,
        "chat_template_kwargs": {"enable_thinking": enable_thinking},
    }))
}

fn run_tokenize_request(
    endpoint: LocalEndpoint,
    timeout: Duration,
    body: &str,
) -> Result<Tokenization, RuntimeError> {
    let value = post_json_request(endpoint, timeout, "/tokenize", body)?;
    let tokens = value
        .get("tokens")
        .and_then(Value::as_array)
        .ok_or_else(|| RuntimeError::Protocol("tokenize response missing tokens array".into()))?;

    Ok(Tokenization {
        token_count: tokens.len(),
    })
}

fn post_json_request(
    endpoint: LocalEndpoint,
    timeout: Duration,
    path: &str,
    body: &str,
) -> Result<Value, RuntimeError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.socket_addr(), Duration::from_secs(3))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        endpoint.port,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| RuntimeError::Protocol(status_line.trim().into()))?;

    let mut chunked = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(RuntimeError::Protocol("unexpected EOF in headers".into()));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        }
    }

    let response_body = if chunked {
        String::from_utf8(read_chunked_body(&mut reader)?)
            .map_err(|error| RuntimeError::Protocol(format!("non-UTF8 JSON body: {error}")))?
    } else {
        let mut body = String::new();
        reader.read_to_string(&mut body)?;
        body
    };

    if !(200..300).contains(&status) {
        return Err(RuntimeError::HttpStatus {
            status,
            body: response_body.trim().to_string(),
        });
    }

    serde_json::from_str(&response_body)
        .map_err(|error| RuntimeError::Protocol(format!("invalid JSON response: {error}")))
}

fn read_chunked_body<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, RuntimeError> {
    let mut output = Vec::new();

    loop {
        let mut size_line = String::new();
        if reader.read_line(&mut size_line)? == 0 {
            return Err(RuntimeError::Protocol(
                "unexpected EOF before final HTTP chunk".into(),
            ));
        }

        let size_text = size_line.trim().split(';').next().unwrap_or("");
        if size_text.is_empty() {
            continue;
        }

        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| RuntimeError::Protocol(format!("bad chunk size {size_text}")))?;

        if size == 0 {
            break;
        }

        let start = output.len();
        output.resize(start + size, 0);
        reader.read_exact(&mut output[start..])?;

        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf)?;
        if crlf != *b"\r\n" {
            return Err(RuntimeError::Protocol(
                "HTTP chunk missing CRLF terminator".into(),
            ));
        }
    }

    Ok(output)
}

fn run_stream_request(
    endpoint: LocalEndpoint,
    timeout: Duration,
    body: &str,
    tx: &mpsc::Sender<Result<GenerationChunk, ModelError>>,
) -> Result<(), RuntimeError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.socket_addr(), Duration::from_secs(3))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let request = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        endpoint.port,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u16>().ok())
        .ok_or_else(|| RuntimeError::Protocol(status_line.trim().into()))?;
    let mut chunked = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(RuntimeError::Protocol("unexpected EOF in headers".into()));
        }
        if line.eq("\r\n") || line.eq("\n") {
            break;
        }
        if line.to_ascii_lowercase().starts_with("transfer-encoding:")
            && line.to_ascii_lowercase().contains("chunked")
        {
            chunked = true;
        }
    }
    if !(200..300).contains(&status) {
        let mut error_body = String::new();
        reader.read_to_string(&mut error_body)?;
        return Err(RuntimeError::HttpStatus {
            status,
            body: error_body.trim().to_string(),
        });
    }
    if chunked {
        read_chunked_sse(&mut reader, tx)
    } else {
        read_plain_sse(&mut reader, tx)
    }
}

fn read_plain_sse<R: BufRead>(
    reader: &mut R,
    tx: &mpsc::Sender<Result<GenerationChunk, ModelError>>,
) -> Result<(), RuntimeError> {
    let mut line = String::new();
    while reader.read_line(&mut line)? != 0 {
        consume_sse_line(line.trim_end_matches(['\r', '\n']), tx)?;
        line.clear();
    }
    Ok(())
}

fn read_chunked_sse<R: BufRead>(
    reader: &mut R,
    tx: &mpsc::Sender<Result<GenerationChunk, ModelError>>,
) -> Result<(), RuntimeError> {
    loop {
        let mut size_line = String::new();
        if reader.read_line(&mut size_line)? == 0 {
            break;
        }
        let size_text = size_line.trim().split(';').next().unwrap_or("");
        if size_text.is_empty() {
            continue;
        }
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| RuntimeError::Protocol(format!("bad chunk size {size_text}")))?;
        if size == 0 {
            break;
        }
        let mut buf = vec![0u8; size];
        reader.read_exact(&mut buf)?;
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf)?;
        let text = String::from_utf8_lossy(&buf);
        for line in text.lines() {
            consume_sse_line(line.trim_end_matches('\r'), tx)?;
        }
    }
    Ok(())
}

fn consume_sse_line(
    line: &str,
    tx: &mpsc::Sender<Result<GenerationChunk, ModelError>>,
) -> Result<(), RuntimeError> {
    let Some(data) = line.strip_prefix("data:") else {
        return Ok(());
    };
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }
    let value: Value = serde_json::from_str(data)
        .map_err(|e| RuntimeError::Protocol(format!("invalid SSE JSON: {e}")))?;
    let reasoning = value
        .pointer("/choices/0/delta/reasoning_content")
        .or_else(|| value.pointer("/choices/0/message/reasoning_content"))
        .and_then(Value::as_str);
    if let Some(reasoning) = reasoning {
        if !reasoning.is_empty() {
            tx.send(Ok(GenerationChunk::Reasoning(reasoning.to_string())))
                .map_err(|_| RuntimeError::Protocol("stream consumer dropped".into()))?;
        }
    }
    let content = value
        .pointer("/choices/0/delta/content")
        .or_else(|| value.pointer("/choices/0/message/content"))
        .and_then(Value::as_str);
    if let Some(content) = content {
        if !content.is_empty() {
            tx.send(Ok(GenerationChunk::Text(content.to_string())))
                .map_err(|_| RuntimeError::Protocol("stream consumer dropped".into()))?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ManagedRuntime {
    command: String,
    args: Vec<String>,
    endpoint: LocalEndpoint,
    child: Arc<Mutex<Option<Child>>>,
}
impl ManagedRuntime {
    pub fn new(command: impl Into<String>, args: Vec<String>, endpoint: LocalEndpoint) -> Self {
        Self {
            command: command.into(),
            args,
            endpoint,
            child: Arc::new(Mutex::new(None)),
        }
    }
    pub fn ensure_started(&self) -> Result<(), RuntimeError> {
        if OpenAiLocalClient::new(self.endpoint).is_ready() {
            return Ok(());
        }
        let should_spawn = {
            let mut guard = self
                .child
                .lock()
                .map_err(|_| RuntimeError::Spawn("runtime lock poisoned".into()))?;
            match guard.as_mut() {
                Some(child) => {
                    if child.try_wait()?.is_some() {
                        *guard = None;
                        true
                    } else {
                        false
                    }
                }
                None => true,
            }
        };
        if should_spawn {
            let child = Command::new(&self.command)
                .args(&self.args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| RuntimeError::Spawn(format!("{}: {e}", self.command)))?;
            let mut guard = self
                .child
                .lock()
                .map_err(|_| RuntimeError::Spawn("runtime lock poisoned".into()))?;
            *guard = Some(child);
        }
        self.wait_ready()
    }

    fn exited_status(&self) -> Result<Option<(std::process::ExitStatus, String)>, RuntimeError> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| RuntimeError::Spawn("runtime lock poisoned".into()))?;
        let Some(child) = guard.as_mut() else {
            return Ok(None);
        };
        let Some(status) = child.try_wait()? else {
            return Ok(None);
        };
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            pipe.read_to_string(&mut stderr)?;
        }
        *guard = None;
        Ok(Some((status, stderr.trim().to_string())))
    }
    fn wait_ready(&self) -> Result<(), RuntimeError> {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            if let Some((status, stderr)) = self.exited_status()? {
                let detail = if stderr.is_empty() {
                    format!("runtime exited during model load with status {status}")
                } else {
                    format!("runtime exited during model load with status {status}: {stderr}")
                };
                return Err(RuntimeError::Spawn(detail));
            }
            match probe_health(self.endpoint) {
                Ok(response) if response.status == 200 => return Ok(()),
                Ok(response) if response.status == 503 => {}
                Ok(response) => {
                    return Err(RuntimeError::HttpStatus {
                        status: response.status,
                        body: response.body,
                    });
                }
                Err(RuntimeError::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::UnexpectedEof
                    ) => {}
                Err(error) => return Err(error),
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(RuntimeError::Spawn(
            "runtime did not become healthy before model-load timeout".into(),
        ))
    }
    pub fn stop(&self) {
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut c) = g.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}
impl Drop for ManagedRuntime {
    fn drop(&mut self) {
        if Arc::strong_count(&self.child) == 1 {
            self.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn fake_reasoning_server() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let payload = concat!(
                "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"PASS\"}}]}\n\n",
                "data: [DONE]\n\n"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/event-stream\r\n\r\n{}",
                payload.len(),
                payload
            )
            .unwrap();
        });
        port
    }

    fn fake_server(chunked: bool) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let p = l.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                if r.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: [DONE]\n\n";
            if chunked {
                write!(s,"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/event-stream\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",payload.len(),payload).unwrap();
            } else {
                write!(s,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/event-stream\r\n\r\n{}",payload.len(),payload).unwrap();
            }
        });
        p
    }
    fn req() -> GenerationRequest {
        GenerationRequest {
            model: "local".into(),
            messages: vec![aether_core::Message::new(Role::User, "ping")],
            settings: aether_core::GenerationSettings::default(),
        }
    }
    #[test]
    fn reasoning_stream_is_not_lost_or_folded_into_final_text() {
        let rx = OpenAiLocalClient::new(LocalEndpoint::new(fake_reasoning_server()))
            .generate_stream(req())
            .unwrap();
        let chunks: Vec<_> = rx.into_iter().map(|chunk| chunk.unwrap()).collect();
        let final_text = chunks
            .iter()
            .filter_map(|chunk| match chunk {
                GenerationChunk::Text(text) => Some(text.as_str()),
                GenerationChunk::Reasoning(_) => None,
            })
            .collect::<String>();
        assert_eq!(final_text, "PASS");
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn wait_ready_requires_health_200_not_just_open_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let health_requests = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&health_requests);
        thread::spawn(move || {
            for status in [503u16, 503, 200] {
                loop {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut first_line = String::new();
                    if reader.read_line(&mut first_line).unwrap() == 0 {
                        continue;
                    }
                    let mut line = String::new();
                    loop {
                        line.clear();
                        if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                            break;
                        }
                    }
                    if !first_line.starts_with("GET /health ") {
                        continue;
                    }
                    seen.fetch_add(1, Ordering::SeqCst);
                    let body = if status == 200 {
                        r#"{"status":"ok"}"#
                    } else {
                        r#"{"error":{"code":503,"message":"Loading model","type":"unavailable_error"}}"#
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} TEST\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    break;
                }
            }
        });

        let runtime = ManagedRuntime::new("unused", vec![], LocalEndpoint::new(port));
        runtime.wait_ready().unwrap();
        assert_eq!(health_requests.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn wait_ready_reports_runtime_exit_without_waiting_for_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = ManagedRuntime::new(
            "/bin/sh",
            vec!["-c".into(), "exit 23".into()],
            LocalEndpoint::new(port),
        );
        let child = Command::new("/bin/sh")
            .args(["-c", "exit 23"])
            .spawn()
            .unwrap();
        *runtime.child.lock().unwrap() = Some(child);

        let cloned = runtime.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(cloned.wait_ready());
        });

        let result = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("runtime exit should be reported before the load timeout");
        match result {
            Err(RuntimeError::Spawn(message)) => {
                assert!(message.contains("23"), "unexpected message: {message}");
            }
            other => panic!("expected runtime exit error, got {other:?}"),
        }
    }

    #[test]
    fn managed_runtime_exit_includes_captured_stderr() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let runtime = ManagedRuntime::new(
            "/bin/sh",
            vec!["-c".into(), "echo fatal-model-load >&2; exit 23".into()],
            LocalEndpoint::new(port),
        );
        let error = runtime.ensure_started().unwrap_err();
        let message = error.to_string();
        assert!(message.contains("23"), "unexpected exit error: {message}");
        assert!(
            message.contains("fatal-model-load"),
            "stderr was not preserved: {message}"
        );
    }

    #[test]
    fn health_error_preserves_server_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = r#"{"error":{"code":500,"message":"model failed to load"}}"#;
            write!(
                stream,
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let runtime = ManagedRuntime::new("unused", vec![], LocalEndpoint::new(port));
        let error = runtime.wait_ready().unwrap_err();
        assert!(
            error.to_string().contains("model failed to load"),
            "unexpected health error: {error}"
        );
    }

    #[test]
    fn generation_http_error_preserves_server_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = r#"{"error":{"message":"model unavailable: insufficient VRAM"}}"#;
            write!(
                stream,
                "HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let rx = OpenAiLocalClient::new(LocalEndpoint::new(port))
            .generate_stream(req())
            .unwrap();
        let error = rx.recv().unwrap().unwrap_err();
        assert!(
            error.to_string().contains("insufficient VRAM"),
            "unexpected provider error: {error}"
        );
    }

    #[test]
    fn parses_plain_sse() {
        let rx = OpenAiLocalClient::new(LocalEndpoint::new(fake_server(false)))
            .generate_stream(req())
            .unwrap();
        let out: Vec<_> = rx.into_iter().map(|x| x.unwrap()).collect();
        assert_eq!(
            out,
            vec![
                GenerationChunk::Text("Hel".into()),
                GenerationChunk::Text("lo".into())
            ]
        );
    }
    #[test]
    fn balanced_request_disables_qwen_thinking() {
        let body = request_json(&req()).unwrap();
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            value.pointer("/chat_template_kwargs/enable_thinking"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn deep_request_enables_qwen_thinking() {
        let mut request = req();
        request.settings =
            aether_core::GenerationSettings::from_preset(aether_core::ProcessingPreset::Deep);
        let body = request_json(&request).unwrap();
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            value.pointer("/chat_template_kwargs/enable_thinking"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn parses_chunked_sse() {
        let rx = OpenAiLocalClient::new(LocalEndpoint::new(fake_server(true)))
            .generate_stream(req())
            .unwrap();
        let text = rx
            .into_iter()
            .filter_map(|x| match x.unwrap() {
                GenerationChunk::Text(t) => Some(t),
                GenerationChunk::Reasoning(_) => None,
            })
            .collect::<String>();
        assert_eq!(text, "Hello");
    }
}
pub mod runtime;
