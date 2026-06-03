use std::{net::TcpListener, thread, time::Duration};

use super::*;

#[test]
fn endpoint_normalizes_relative_paths() {
    let client =
        HttpReleaseClient::new("https://releases.example.com/").expect("client should build");

    assert_eq!(
        client
            .endpoint("releases/download/example-cli/1.2.3/aarch64-apple-darwin")
            .expect("relative URL should normalize"),
        "https://releases.example.com/releases/download/example-cli/1.2.3/aarch64-apple-darwin"
    );
    assert_eq!(
        client
            .endpoint("/releases/latest/example-cli/aarch64-apple-darwin")
            .expect("leading slash URL should remain valid"),
        "https://releases.example.com/releases/latest/example-cli/aarch64-apple-darwin"
    );
}

#[tokio::test]
async fn custom_timeout_bounds_stalled_release_request() {
    let server_url = hanging_http_server();
    let client = HttpReleaseClient::new_with_timeouts(
        server_url,
        Duration::from_millis(50),
        Duration::from_millis(50),
    )
    .expect("client should build");

    let error = client
        .latest("example-cli", "aarch64-apple-darwin")
        .await
        .expect_err("stalled request should time out");

    assert!(matches!(error, Error::ReleaseServerRequest { .. }));
}

fn hanging_http_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let address = listener.local_addr().expect("listener should have address");
    thread::spawn(move || {
        if let Ok((_stream, _address)) = listener.accept() {
            thread::sleep(Duration::from_secs(2));
        }
    });
    format!("http://{address}")
}
