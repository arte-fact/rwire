//! A `Router` needs an `outlet()` to render into. `run()` checks the page tree at
//! startup — including regions that only appear inside another renderer's output —
//! and refuses to start without one, instead of serving a page whose navigation
//! freezes. With an outlet (or without a router), the server starts as before.

use async_std::io::ReadExt;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;
use rwire::{
    el, outlet, renderer, CurrentRoute, El, ElementBuilder, Router, Server, State,
    ROUTER_WITHOUT_OUTLET,
};
use std::time::Duration;

fn router() -> Router {
    Router::new()
        .page("/", |_| el(El::P).text("home"))
        .page("/about", |_| el(El::P).text("about"))
}

fn shell_without_outlet() -> ElementBuilder {
    el(El::Div).append([el(El::H1).text("no outlet here")])
}

/// A nav highlight reads `CurrentRoute` too, but it is not the outlet.
#[renderer]
fn nav(route: &CurrentRoute) -> ElementBuilder {
    el(El::Nav).text(route.path())
}

fn shell_with_only_a_route_reader() -> ElementBuilder {
    el(El::Div).append([nav()])
}

fn shell_with_outlet() -> ElementBuilder {
    el(El::Div).append([nav(), el(El::Main).append([outlet()])])
}

#[derive(State, Default)]
#[storage(memory)]
struct Layout {
    wide: bool,
}

/// The outlet lives inside a renderer's output, not in the static tree.
#[renderer]
fn layout(state: &Layout) -> ElementBuilder {
    el(El::Div)
        .text(if state.wide { "wide" } else { "narrow" })
        .append([outlet()])
}

fn shell_with_nested_outlet() -> ElementBuilder {
    el(El::Div).append([layout()])
}

/// `run()` must fail at once with the explicit startup error.
async fn assert_refuses(port: u16, root: fn() -> ElementBuilder) {
    let run = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(root)
        .routes(router())
        .run();
    let result = async_std::future::timeout(Duration::from_secs(5), run)
        .await
        .expect("run() kept serving instead of refusing to start");
    let err = result.expect_err("a router without an outlet must not start");
    assert_eq!(err.to_string(), ROUTER_WITHOUT_OUTLET);
}

/// The server must start and serve the page.
async fn assert_serves(port: u16, root: fn() -> ElementBuilder, with_router: bool) {
    let server = task::spawn(async move {
        let builder = Server::bind(&format!("127.0.0.1:{port}"))
            .unwrap()
            .root(root);
        let builder = if with_router {
            builder.routes(router())
        } else {
            builder
        };
        builder.run().await.map_err(|e| e.to_string())
    });
    task::sleep(Duration::from_millis(150)).await;
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("the server did not start");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut head = [0u8; 64];
    let n = stream.read(&mut head).await.unwrap();
    assert!(
        head[..n].starts_with(b"HTTP/1.1 200 OK"),
        "{}",
        String::from_utf8_lossy(&head[..n])
    );
    drop(stream);
    server.cancel().await;
}

#[async_std::test]
async fn router_without_outlet_refuses_to_start() {
    assert_refuses(19041, shell_without_outlet).await;
}

#[async_std::test]
async fn a_current_route_reader_is_not_an_outlet() {
    assert_refuses(19042, shell_with_only_a_route_reader).await;
}

#[async_std::test]
async fn router_with_outlet_starts() {
    assert_serves(19043, shell_with_outlet, true).await;
}

#[async_std::test]
async fn an_outlet_inside_a_renderer_is_found() {
    assert_serves(19044, shell_with_nested_outlet, true).await;
}

#[async_std::test]
async fn no_router_needs_no_outlet() {
    assert_serves(19045, shell_without_outlet, false).await;
}
