use aether_model_runtime::{LocalEndpoint, OpenAiLocalClient};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn local_client_posts_llama_tokenize_and_counts_real_tokens() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        assert!(first.starts_with("POST /tokenize HTTP/1.1"));

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line
                .to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
            {
                content_length = value;
            }
        }

        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).unwrap();
        let body = String::from_utf8(body).unwrap();
        assert!(body.contains("\"content\":\"one two three\""));

        let response = r#"{"tokens":[151646,825,1378,2201]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.len(),
            response
        )
        .unwrap();
    });

    let result = OpenAiLocalClient::new(LocalEndpoint::new(port))
        .tokenize("one two three")
        .unwrap();

    assert_eq!(result.token_count, 4);
}
