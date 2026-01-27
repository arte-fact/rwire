//! Counter example - demonstrates wire-wasm reactive state API.
//!
//! A simple counter component with increment/decrement buttons.
//! Only the count display re-renders when state changes.

use wire_wasm::{el, handler, renderer, ClientState, El, ElementBuilder, Ev, Server};

#[derive(ClientState, Default)]
struct Counter {
    count: i32,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("wire-wasm Server - Counter Example");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    Server::bind("127.0.0.1:9000")?
        .root(build_counter)
        .run()
        .await
}

fn build_counter() -> ElementBuilder {
    el(El::Div).class("counter").append([
        el(El::Button).text("-").on(Ev::Click, decrement()),
        render_count(),
        el(El::Button).text("+").on(Ev::Click, increment()),
    ])
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    el(El::Span).text(&state.count.to_string())
}

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}
