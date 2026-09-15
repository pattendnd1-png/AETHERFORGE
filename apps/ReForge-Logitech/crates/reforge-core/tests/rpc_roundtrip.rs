use reforge_core::{client, RpcData, RpcRequest, RpcResponse};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    thread,
};

#[test]
fn unix_rpc_uses_one_json_request_per_line() {
    let socket = std::env::temp_dir().join(format!(
        "reforge-logitech-rpc-test-{}.sock",
        std::process::id()
    ));
    let _ = fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert!(line.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<RpcRequest>(line.trim_end()).unwrap(),
            RpcRequest::Health
        );

        let response = RpcResponse::ok(RpcData::Health {
            version: "test".to_owned(),
        });
        serde_json::to_writer(&mut stream, &response).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
    });

    let response = client::call_at(&socket, &RpcRequest::Health).unwrap();
    assert_eq!(
        response,
        RpcResponse::ok(RpcData::Health {
            version: "test".to_owned(),
        })
    );

    server.join().unwrap();
    fs::remove_file(&socket).unwrap();
}
