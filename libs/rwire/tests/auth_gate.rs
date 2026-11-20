//! The form-login gate (`Server::auth`) over a real socket.
//!
//! A reverse proxy (or a slow client) often forwards a `POST /login` in several TCP
//! segments: the head first, the form body a moment later. The gate peeks the first
//! segment to route the request, so it must keep reading until the declared body is
//! complete before checking the credentials — otherwise valid credentials fail.

use async_std::io::ReadExt;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;
use rwire::{el, El, ElementBuilder, Server};
use std::time::Duration;

const USER: &str = "admin";
const PASSWORD: &str = "secret";

fn build_simple() -> ElementBuilder {
    el(El::Div).text("behind the gate")
}

/// Start a gated server on `port` and give it time to bind.
async fn start_gated(port: u16) -> task::JoinHandle<()> {
    let handle = task::spawn(async move {
        let _ = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(build_simple)
            .auth(USER, PASSWORD)
            .run()
            .await;
    });
    task::sleep(Duration::from_millis(150)).await;
    handle
}

/// Send `segments` one write at a time, pausing between them so each lands as its own
/// TCP read on the server, then read the response head (the server closes after it).
async fn post_in_segments(port: u16, segments: &[&[u8]]) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream.set_nodelay(true).unwrap();
    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            task::sleep(Duration::from_millis(200)).await;
        }
        stream.write_all(segment).await.unwrap();
        stream.flush().await.unwrap();
    }
    let mut response = Vec::new();
    let read = async_std::io::timeout(Duration::from_secs(5), async {
        stream.read_to_end(&mut response).await
    })
    .await;
    assert!(read.is_ok(), "the gate never answered: {read:?}");
    String::from_utf8_lossy(&response).into_owned()
}

fn login_head(body: &str) -> String {
    format!(
        "POST /login HTTP/1.1\r\n\
         Host: localhost\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\n\r\n",
        body.len()
    )
}

fn assert_logged_in(response: &str) {
    assert!(
        response.starts_with("HTTP/1.1 303"),
        "expected the post-login redirect, got:\n{response}"
    );
    assert!(
        response.contains("Set-Cookie: rwire_auth=") && !response.contains("rwire_auth=;"),
        "expected a session cookie, got:\n{response}"
    );
}

/// Head in one segment, form body in the next: the login must succeed.
#[async_std::test]
async fn login_succeeds_when_the_body_arrives_after_the_head() {
    let server = start_gated(19031).await;
    let body = format!("username={USER}&password={PASSWORD}");
    let head = login_head(&body);
    let response = post_in_segments(19031, &[head.as_bytes(), body.as_bytes()]).await;
    assert_logged_in(&response);
    server.cancel().await;
}

/// The head itself split after the request line, then the body split in two: the gate
/// reads until the head is closed and every declared body byte is in.
#[async_std::test]
async fn login_succeeds_when_head_and_body_are_both_fragmented() {
    let server = start_gated(19032).await;
    let body = format!("username={USER}&password={PASSWORD}");
    let head = login_head(&body);
    let (line, rest) = head.split_at(head.find("\r\n").unwrap() + 2);
    let (body_a, body_b) = body.split_at(body.len() / 2);
    let response = post_in_segments(
        19032,
        &[
            line.as_bytes(),
            rest.as_bytes(),
            body_a.as_bytes(),
            body_b.as_bytes(),
        ],
    )
    .await;
    assert_logged_in(&response);
    server.cancel().await;
}

/// Control: fragmentation must not turn wrong credentials into a session.
#[async_std::test]
async fn fragmented_login_with_a_wrong_password_is_refused() {
    let server = start_gated(19033).await;
    let body = format!("username={USER}&password=wrong");
    let head = login_head(&body);
    let response = post_in_segments(19033, &[head.as_bytes(), body.as_bytes()]).await;
    assert!(
        !response.contains("Set-Cookie: rwire_auth="),
        "a wrong password must not mint a session:\n{response}"
    );
    assert!(
        !response.starts_with("HTTP/1.1 303"),
        "a wrong password must not redirect into the app:\n{response}"
    );
    server.cancel().await;
}
