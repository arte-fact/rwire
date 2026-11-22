//! The audio socket, end to end, against a running server.
//!
//! - a page renders a single-use ticket; with it (and the page's session
//!   cookie) a client opens `/_rw/ext/audio/ws`, sends frames, and gets them
//!   back through `link.output` (an echo);
//! - a replayed, unknown or missing ticket, or a ticket presented with
//!   another session, gets the socket closed with 4403;
//! - each connection hands over the next ticket: a lost socket reconnects
//!   with it, no render needed, and the new link has the same session,
//!   name and page connection; another tab has another connection id;
//! - the link's task can change a shared state and the page re-renders;
//! - when the page connection ends, its audio socket is closed and the
//!   ticket it handed over is refused.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::ext_socket::tokio_tungstenite::{
    self,
    tungstenite::{client::IntoClientRequest, Message},
};
use rwire::{el, renderer, El, ElementBuilder, Server, State};
use rwire_audio::wire::{Frame, ToBrowser};
use rwire_audio::{Audio, AudioExt, AudioLink, ServeAudio, AUDIO_WS_PATH, FRAME_SAMPLES};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// What the audio task saw, for the page to show.
#[derive(State, Default)]
#[storage(shared)]
struct Heard {
    frames: u32,
}

#[renderer]
fn heard(state: &Heard) -> ElementBuilder {
    el(El::P).text(&format!("heard: {}", state.frames))
}

fn page() -> ElementBuilder {
    el(El::Div).append([el(El::Div).audio(Audio::new("voice")), heard()])
}

/// Who a link said it was.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    session: String,
    name: String,
    connection: u64,
}

/// A port nobody listens on (fixed ports collide with other checkouts
/// running the same tests on the machine).
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A server whose audio task echoes every frame and counts it in `Heard`.
async fn start(port: u16) -> mpsc::UnboundedReceiver<Seen> {
    let (tx, rx) = mpsc::unbounded_channel();
    let server = Server::bind(&format!("127.0.0.1:{port}"))
        .unwrap()
        .root(page)
        .serve_audio(move |mut link: AudioLink| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(Seen {
                    session: link.session().to_string(),
                    name: link.name().to_string(),
                    connection: link.connection_id(),
                });
                while let Some(frame) = link.input.recv().await {
                    link.output.pcm(&frame.samples, 16_000).unwrap();
                    link.server().update_shared::<Heard>(|h| h.frames += 1);
                }
            }
        });
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    for _ in 0..100 {
        if TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return rx;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server on {port} never started");
}

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

const SID: &str = "0123456789abcdef0123456789abcdef";
const OTHER_SID: &str = "fedcba9876543210fedcba9876543210";

async fn connect(port: u16, path: &str, sid: &str) -> Ws {
    let mut req = format!("ws://127.0.0.1:{port}{path}")
        .into_client_request()
        .unwrap();
    req.headers_mut()
        .insert("Cookie", format!("rwire_sid={sid}").parse().unwrap());
    tokio_tungstenite::connect_async(req).await.unwrap().0
}

async fn audio(port: u16, ticket: &str, sid: &str) -> Ws {
    connect(port, &format!("{AUDIO_WS_PATH}?t={ticket}"), sid).await
}

/// The first ticket a page's frames carry.
async fn rendered_ticket(page: &mut Ws) -> String {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = page.next().await {
            if let Message::Binary(data) = message {
                if let Some(t) = find_ticket(&data) {
                    return Some(t);
                }
            }
        }
        None
    })
    .await;
    found
        .ok()
        .flatten()
        .expect("no ticket in the page's render")
}

fn find_ticket(data: &[u8]) -> Option<String> {
    let at = data.windows(4).position(|w| w == b"rwt_")?;
    let hex = data.get(at + 4..at + 36)?;
    hex.iter()
        .all(u8::is_ascii_hexdigit)
        .then(|| String::from_utf8_lossy(&data[at..at + 36]).into_owned())
}

/// Until a page frame contains `needle`.
async fn expect_page(page: &mut Ws, needle: &str) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = page.next().await {
            if let Message::Binary(data) = message {
                if data.windows(needle.len()).any(|w| w == needle.as_bytes()) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert_eq!(found, Ok(true), "{needle:?} never reached the page");
}

async fn next_message(ws: &mut Ws) -> Option<ToBrowser> {
    let got = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            match message {
                Message::Binary(data) => return ToBrowser::decode(&data),
                Message::Close(_) => return None,
                _ => {}
            }
        }
        None
    })
    .await;
    got.expect("the audio socket said nothing")
}

/// The close code the server ends the socket with.
async fn close_code(ws: &mut Ws) -> Option<u16> {
    let got = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = ws.next().await {
            match message {
                Ok(Message::Close(frame)) => return frame.map(|f| u16::from(f.code)),
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    })
    .await;
    got.expect("the audio socket was never closed")
}

fn frame(seq: u32) -> Frame {
    let mut samples = [0i16; FRAME_SAMPLES];
    for (i, s) in samples.iter_mut().enumerate() {
        *s = (seq as i16 * 1000).wrapping_add(i as i16);
    }
    Frame {
        seq,
        timestamp: f64::from(seq) * 0.02,
        samples,
    }
}

#[tokio::test]
async fn frames_echo_through_the_audio_socket_and_tickets_are_single_use() {
    let port = free_port();
    let mut seen = start(port).await;
    let mut page = connect(port, "/", SID).await;
    let ticket = rendered_ticket(&mut page).await;

    let mut link = audio(port, &ticket, SID).await;
    let Some(ToBrowser::Ticket(next)) = next_message(&mut link).await else {
        panic!("a connection starts with the next ticket");
    };
    assert_ne!(next, ticket);
    let first = seen.recv().await.unwrap();
    assert_eq!(
        (first.session.as_str(), first.name.as_str()),
        (SID, "voice")
    );

    for seq in 0..3 {
        link.send(Message::Binary(frame(seq).encode()))
            .await
            .unwrap();
    }
    for seq in 0..3 {
        assert_eq!(
            next_message(&mut link).await,
            Some(ToBrowser::Pcm {
                rate: 16_000,
                samples: frame(seq).samples.to_vec()
            }),
            "frame {seq} echoed"
        );
    }
    // The task changed a shared state: the page shows it.
    expect_page(&mut page, "heard: 3").await;

    // Replayed, unknown, missing: refused.
    assert_eq!(
        close_code(&mut audio(port, &ticket, SID).await).await,
        Some(4403)
    );
    let forged = format!("rwt_{}", "0".repeat(32));
    assert_eq!(
        close_code(&mut audio(port, &forged, SID).await).await,
        Some(4403)
    );
    let mut bare = connect(port, AUDIO_WS_PATH, SID).await;
    assert_eq!(close_code(&mut bare).await, Some(4403));

    // Another tab of the session: its own ticket and connection id. With
    // another session's cookie, the ticket is refused (and burnt).
    let mut tab = connect(port, "/", SID).await;
    let tab_ticket = rendered_ticket(&mut tab).await;
    assert_ne!(tab_ticket, ticket);
    assert_eq!(
        close_code(&mut audio(port, &tab_ticket, OTHER_SID).await).await,
        Some(4403)
    );
    assert_eq!(
        close_code(&mut audio(port, &tab_ticket, SID).await).await,
        Some(4403),
        "a ticket is burnt by any use"
    );
    let mut tab = connect(port, "/", SID).await;
    let tab_ticket = rendered_ticket(&mut tab).await;
    let mut tab_link = audio(port, &tab_ticket, SID).await;
    assert!(matches!(
        next_message(&mut tab_link).await,
        Some(ToBrowser::Ticket(_))
    ));
    let other = seen.recv().await.unwrap();
    assert_ne!(other.connection, first.connection);

    // The socket is lost; the next ticket reconnects it, no render.
    drop(link);
    let mut again = audio(port, &next, SID).await;
    let Some(ToBrowser::Ticket(after)) = next_message(&mut again).await else {
        panic!("a reconnection starts with the next ticket");
    };
    assert_ne!(after, next);
    assert_eq!(
        seen.recv().await.unwrap(),
        first,
        "same session, name, page"
    );
    again
        .send(Message::Binary(frame(9).encode()))
        .await
        .unwrap();
    assert!(matches!(
        next_message(&mut again).await,
        Some(ToBrowser::Pcm { .. })
    ));
    // The next ticket is single-use too.
    assert_eq!(
        close_code(&mut audio(port, &next, SID).await).await,
        Some(4403)
    );
}

#[tokio::test]
async fn the_end_of_the_page_connection_closes_its_audio_socket_and_burns_its_tickets() {
    let port = free_port();
    let _seen = start(port).await;
    let mut page = connect(port, "/", SID).await;
    let ticket = rendered_ticket(&mut page).await;
    let mut link = audio(port, &ticket, SID).await;
    let Some(ToBrowser::Ticket(next)) = next_message(&mut link).await else {
        panic!("a connection starts with the next ticket");
    };
    drop(page);
    // The audio socket ends (closed or dropped) without the browser asking.
    let ended = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = link.next().await {
            if let Message::Close(_) = message {
                break;
            }
        }
    })
    .await;
    assert!(ended.is_ok(), "the audio socket outlived its page");
    assert_eq!(
        close_code(&mut audio(port, &next, SID).await).await,
        Some(4403),
        "the handed-over ticket died with its page"
    );
}
