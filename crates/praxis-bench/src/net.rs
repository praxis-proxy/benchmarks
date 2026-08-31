//! Network utilities for benchmark orchestration.

use std::time::Duration;

use crate::error::BenchmarkError;

// -----------------------------------------------------------------------------
// Network Constants
// -----------------------------------------------------------------------------

/// Interval between health check polls.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(250);

// -----------------------------------------------------------------------------
// TCP Readiness
// -----------------------------------------------------------------------------

/// Poll a TCP address until a connection succeeds or timeout.
pub(crate) async fn wait_for_tcp(addr: &str, timeout: Duration) -> Result<(), BenchmarkError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(BenchmarkError::ToolFailed {
                tool: "health_check".into(),
                code: -1,
                stderr: format!("timeout waiting for TCP on {addr}"),
            });
        }

        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }

        tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
    }
}

// -----------------------------------------------------------------------------
// HTTP Readiness
// -----------------------------------------------------------------------------

/// Poll an HTTP URL until it returns 200 or timeout.
pub(crate) async fn wait_for_http(url: &str, timeout: Duration) -> Result<(), BenchmarkError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(BenchmarkError::ToolFailed {
                tool: "health_check".into(),
                code: -1,
                stderr: format!("timeout waiting for HTTP on {url}"),
            });
        }

        if let Ok(resp) = simple_http_get(url).await
            && (resp.starts_with("HTTP/1.1 200") || resp.starts_with("HTTP/1.0 200"))
        {
            return Ok(());
        }

        tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
    }
}

/// Minimal HTTP GET using raw TCP (no external dependency).
async fn simple_http_get(url: &str) -> Result<String, BenchmarkError> {
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = stripped.split_once('/').unwrap_or((stripped, ""));

    let mut stream = tokio::net::TcpStream::connect(host_port).await?;

    let request = format!(
        "GET /{path} HTTP/1.1\r\n\
         Host: {host_port}\r\n\
         Connection: close\r\n\r\n"
    );

    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes()).await?;

    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf).await?;

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

// -----------------------------------------------------------------------------
// Docker Cleanup
// -----------------------------------------------------------------------------

/// Stop and remove a Docker container by name.
pub(crate) async fn stop_container(name: &str) {
    let _status = tokio::process::Command::new("docker")
        .args(["rm", "-f", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

// -----------------------------------------------------------------------------
// Git Commit Detection
// -----------------------------------------------------------------------------

/// Detect the current git commit SHA.
pub fn detect_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            o.status
                .success()
                .then(|| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".into())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, reason = "tests")]
mod tests {
    use super::*;

    /// Bind a listener and spawn a task that answers one request with the
    /// given raw HTTP response, returning the bound address.
    async fn spawn_http_once(response: &'static [u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
                let mut buf = [0_u8; 1024];
                let _read = sock.read(&mut buf).await;
                let _written = sock.write_all(response).await;
            }
        });
        addr.to_string()
    }

    #[tokio::test]
    async fn wait_for_tcp_returns_ok_when_listening() {
        // A bound listener accepts the connection, so wait_for_tcp succeeds
        // well within the timeout. A past-deadline mutant (`+` -> `-`) would
        // instead time out immediately.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        wait_for_tcp(&addr.to_string(), Duration::from_secs(5))
            .await
            .expect("connection to a bound listener should succeed");
    }

    #[tokio::test]
    async fn wait_for_tcp_times_out_with_zero_deadline() {
        // A zero timeout puts the deadline at "now", so the very first check
        // returns the timeout error before any connect attempt.
        let err = wait_for_tcp("127.0.0.1:9", Duration::ZERO)
            .await
            .expect_err("zero timeout should error");
        match err {
            BenchmarkError::ToolFailed { code, tool, .. } => {
                assert_eq!(code, -1, "timeout should report code -1");
                assert_eq!(tool, "health_check", "timeout should tag the health_check tool");
            },
            other => panic!("expected ToolFailed, got {other}"),
        }
    }

    #[tokio::test]
    async fn wait_for_http_returns_ok_on_200() {
        let addr = spawn_http_once(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
        let url = format!("http://{addr}/");
        wait_for_http(&url, Duration::from_secs(5))
            .await
            .expect("a 200 response should satisfy readiness");
    }

    #[tokio::test]
    async fn wait_for_http_accepts_http_1_0() {
        // Exercises the second half of the `||`: an HTTP/1.0 200 is ready too.
        let addr = spawn_http_once(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n").await;
        let url = format!("http://{addr}/");
        wait_for_http(&url, Duration::from_secs(5))
            .await
            .expect("an HTTP/1.0 200 response should satisfy readiness");
    }

    #[tokio::test]
    async fn wait_for_http_times_out_with_zero_deadline() {
        let err = wait_for_http("http://127.0.0.1:9/", Duration::ZERO)
            .await
            .expect_err("zero timeout should error");
        match err {
            BenchmarkError::ToolFailed { code, .. } => assert_eq!(code, -1, "timeout should report code -1"),
            other => panic!("expected ToolFailed, got {other}"),
        }
    }

    #[tokio::test]
    async fn simple_http_get_returns_the_raw_response() {
        let addr = spawn_http_once(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello").await;
        let url = format!("http://{addr}/health");
        let resp = simple_http_get(&url).await.expect("request should succeed");
        assert!(
            resp.starts_with("HTTP/1.1 200"),
            "status line should be preserved: {resp}"
        );
        assert!(resp.contains("hello"), "body should be preserved: {resp}");
    }

    #[test]
    fn detect_commit_is_never_empty_or_a_placeholder() {
        // In a git checkout this is a short SHA; elsewhere it falls back to
        // "unknown". Either way it is non-empty and not the mutant sentinel.
        let commit = detect_commit();
        assert!(!commit.is_empty(), "commit string should never be empty");
        assert_ne!(commit, "xyzzy", "commit string should be real, not a mutant sentinel");
    }
}
