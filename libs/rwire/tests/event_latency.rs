//! An event's answer is two frames (the DOM update, then EVENT_ACK). Without
//! TCP_NODELAY on the server socket, Nagle holds the second until the client
//! ACKs the first, and delayed ACK makes that ~40 ms per click.

use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use rwire::{el, handler, renderer, El, ElementBuilder, Ev, Server, State};
use tokio_tungstenite::tungstenite::Message;

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    n: u32,
}

#[handler]
fn bump(counter: &mut Counter) {
    counter.n += 1;
}

#[renderer]
fn count(counter: &Counter) -> ElementBuilder {
    el(El::P).text(&counter.n.to_string())
}

fn app() -> ElementBuilder {
    el(El::Main).append([count(), el(El::Button).text("+").on(Ev::Click, bump())])
}

#[tokio::test]
async fn a_click_round_trip_does_not_wait_for_delayed_ack() {
    tokio::spawn(async {
        let _ = Server::bind("127.0.0.1:19081")
            .unwrap()
            .root(app)
            .run()
            .await;
    });
    let mut ws = None;
    for _ in 0..100 {
        if let Ok((socket, _)) = tokio_tungstenite::connect_async("ws://127.0.0.1:19081/").await {
            ws = Some(socket);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mut ws = ws.expect("the server never started");
    ws.next().await.unwrap().unwrap(); // initial DOM

    let mut click = vec![0x00];
    rwire::protocol::varint::write_varint(&mut click, bump().handler_id());
    click.extend_from_slice(&[Ev::Click.as_u8(), 0, 0]);

    let mut rounds = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        ws.send(Message::Binary(click.clone())).await.unwrap();
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(5), ws.next())
                .await
                .expect("no answer")
                .unwrap()
                .unwrap();
            // EVENT_ACK + BATCH_END closes every event's answer.
            if let Message::Binary(data) = frame {
                if data.ends_with(&[0xF3, 0xFF]) {
                    break;
                }
            }
        }
        rounds.push(start.elapsed());
    }
    rounds.sort();
    let median = rounds[rounds.len() / 2];
    assert!(
        median < Duration::from_millis(20),
        "median click round trip {median:?}: the answer waits on delayed ACK"
    );
}
