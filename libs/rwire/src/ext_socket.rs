//! Extension sockets: a WebSocket of a runtime extension's own, beside the
//! page's socket, for a stream the event path is not made for (binary
//! media, high-rate frames).
//!
//! - **The route.** [`ServerWithRoot::ext_socket`](crate::ServerWithRoot::ext_socket)
//!   registers a path (`"/_rw/ext/<name>/ws"`). A WebSocket handshake on it
//!   passes the same gates as the page's socket (the auth gate or the
//!   delegated one, the Origin check, the admission caps), then the handler
//!   gets the accepted socket as an [`ExtSocket`].
//! - **The ticket.** The handshake must carry `?t=<ticket>`, a single-use
//!   ticket the page's connection issued while rendering ([`ticket`]). The
//!   ticket is bound to that connection: its session (the cookie of the
//!   handshake must name the same one), its identity under delegated auth,
//!   its connection id, and a label the extension chose (a stream name).
//!   A ticket that is unknown, already used, expired, issued for another
//!   path, or presented with another session or identity gets the socket
//!   closed with [`CLOSE_TICKET_REFUSED`] right after the handshake.
//! - **Reconnecting.** A socket can be lost while the page stays. The
//!   handler asks [`ExtSocket::next_ticket`] for the ticket of the next
//!   connection and sends it to the extension over its own protocol; that
//!   ticket stays valid as long as the socket lives, then
//!   [`TICKET_TTL`] more, so the extension reconnects with no new render.
//!
//! The layer is protocol-free: what travels on the socket is the
//! extension's business. `rwire-audio` is built on it.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
pub use tokio_tungstenite;
use tokio_tungstenite::WebSocketStream;

use crate::delegated_auth::Identity;
use crate::server::SharedServerState;

/// What every ticket starts with (then 32 hex digits).
pub const TICKET_PREFIX: &str = "rwt_";
/// How long an unused ticket stays valid: from its issue for a ticket
/// issued by a render, from the end of its socket for a
/// [`ExtSocket::next_ticket`].
pub const TICKET_TTL: Duration = Duration::from_secs(120);
/// The close code of a socket whose ticket was refused. The extension must
/// not retry with the same ticket: it waits for a render to issue another.
pub const CLOSE_TICKET_REFUSED: u16 = 4403;

/// The page connection a render runs for.
#[derive(Clone)]
pub struct ConnectionInfo {
    /// The page socket's id (unique in the process): two tabs of one
    /// session have two.
    pub connection_id: u64,
    /// The session (the `rwire_sid` cookie).
    pub session: String,
    /// Who the connection belongs to, under delegated auth.
    pub identity: Option<Identity>,
}

struct Scope {
    info: ConnectionInfo,
    shared: Arc<SharedServerState>,
}

tokio::task_local! {
    static SCOPE: OnceLock<Scope>;
}

/// Run a page connection's task with room for its scope ([`enter`]).
pub(crate) async fn with_scope<F: Future>(f: F) -> F::Output {
    SCOPE.scope(OnceLock::new(), f).await
}

/// Record, once, which connection this task serves: every render after it
/// (initial, handler, broadcast) runs on this task and sees it.
pub(crate) fn enter(info: ConnectionInfo, shared: Arc<SharedServerState>) {
    let _ = SCOPE.try_with(|s| s.set(Scope { info, shared }));
}

/// The page connection the current render runs for; `None` outside one
/// (a static paint, a unit test, a background task).
pub fn current_connection() -> Option<ConnectionInfo> {
    SCOPE
        .try_with(|s| s.get().map(|s| s.info.clone()))
        .ok()
        .flatten()
}

/// A ticket for the extension socket at `path`, bound to the current page
/// connection and to `label`, to put in the rendered element. While it is
/// unused, rendering again gives the same ticket (a render stays stable).
/// `None` outside a page connection: the live render that follows a static
/// paint issues it.
pub fn ticket(path: &str, label: &str) -> Option<String> {
    SCOPE
        .try_with(|s| {
            s.get()
                .map(|s| s.shared.tickets.for_render(path, label, &s.info))
        })
        .ok()
        .flatten()
}

/// What a ticket was issued for.
#[derive(Clone)]
pub struct TicketClaim {
    /// The extension socket's path.
    pub path: String,
    /// The label given at issue (a stream name).
    pub label: String,
    /// The page connection that issued it.
    pub connection: ConnectionInfo,
}

struct Entry {
    claim: TicketClaim,
    /// `None` while the socket that asked for it lives.
    expires: Option<Instant>,
    /// Issued by a render (reused by the next render while unused).
    rendered: bool,
}

/// The tickets issued and not yet used.
#[derive(Default)]
pub(crate) struct Tickets {
    map: Mutex<HashMap<String, Entry>>,
}

impl Tickets {
    fn prune(map: &mut HashMap<String, Entry>, now: Instant) {
        map.retain(|_, e| e.expires.is_none_or(|t| t > now));
    }

    fn for_render(&self, path: &str, label: &str, info: &ConnectionInfo) -> String {
        let now = Instant::now();
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        Self::prune(&mut map, now);
        let reusable = map.iter().find(|(_, e)| {
            e.rendered
                && e.claim.path == path
                && e.claim.label == label
                && e.claim.connection.connection_id == info.connection_id
                && e.expires.is_some_and(|t| t > now + TICKET_TTL / 2)
        });
        if let Some((ticket, _)) = reusable {
            return ticket.clone();
        }
        let ticket = new_ticket();
        map.insert(
            ticket.clone(),
            Entry {
                claim: TicketClaim {
                    path: path.to_string(),
                    label: label.to_string(),
                    connection: info.clone(),
                },
                expires: Some(now + TICKET_TTL),
                rendered: true,
            },
        );
        ticket
    }

    fn held(&self, claim: TicketClaim) -> String {
        let ticket = new_ticket();
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        Self::prune(&mut map, Instant::now());
        map.insert(
            ticket.clone(),
            Entry {
                claim,
                expires: None,
                rendered: false,
            },
        );
        ticket
    }

    fn release(&self, ticket: &str) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(e) = map.get_mut(ticket) {
            e.expires = Some(Instant::now() + TICKET_TTL);
        }
    }

    /// Use `ticket` once: it is gone whatever the outcome. The claim, if it
    /// was valid, unexpired and issued for `path`.
    pub(crate) fn redeem(&self, ticket: &str, path: &str) -> Option<TicketClaim> {
        let now = Instant::now();
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.remove(ticket)?;
        Self::prune(&mut map, now);
        (entry.expires.is_none_or(|t| t > now) && entry.claim.path == path).then_some(entry.claim)
    }
}

fn new_ticket() -> String {
    format!(
        "{TICKET_PREFIX}{}",
        crate::session::SessionId::generate().as_str()
    )
}

/// The ticket of a handshake: the `t` parameter of its query.
pub(crate) fn ticket_of(target: &str) -> Option<&str> {
    let (_, query) = target.split_once('?')?;
    query.split('&').find_map(|kv| kv.strip_prefix("t="))
}

/// Whether a claim matches the handshake that presents it.
pub(crate) fn claim_matches(
    claim: &TicketClaim,
    session: Option<&str>,
    identity: Option<&Identity>,
) -> bool {
    session == Some(claim.connection.session.as_str())
        && claim.connection.identity.as_ref().map(Identity::id) == identity.map(Identity::id)
}

/// An accepted extension socket, with what its ticket says about it.
pub struct ExtSocket {
    /// The socket, past the handshake.
    pub ws: WebSocketStream<TcpStream>,
    /// The peer's address.
    pub peer: SocketAddr,
    claim: TicketClaim,
    shared: Arc<SharedServerState>,
}

impl ExtSocket {
    pub(crate) fn new(
        ws: WebSocketStream<TcpStream>,
        peer: SocketAddr,
        claim: TicketClaim,
        shared: Arc<SharedServerState>,
    ) -> Self {
        Self {
            ws,
            peer,
            claim,
            shared,
        }
    }

    /// The label the ticket was issued with.
    pub fn label(&self) -> &str {
        &self.claim.label
    }

    /// The page connection that issued the first ticket of this socket.
    pub fn connection(&self) -> &ConnectionInfo {
        &self.claim.connection
    }

    /// The server's shared state: a socket's task can update a state and
    /// get the pages re-rendered (`update_shared`).
    pub fn shared(&self) -> &Arc<SharedServerState> {
        &self.shared
    }

    /// A ticket for this socket's next connection, with the same claim. It
    /// is valid while the returned guard lives, then [`TICKET_TTL`] more.
    pub fn next_ticket(&self) -> NextTicket {
        NextTicket {
            value: self.shared.tickets.held(self.claim.clone()),
            shared: Arc::clone(&self.shared),
        }
    }
}

/// A ticket for the next connection of a socket (see
/// [`ExtSocket::next_ticket`]). Keep it as long as the socket lives.
pub struct NextTicket {
    value: String,
    shared: Arc<SharedServerState>,
}

impl NextTicket {
    /// The ticket, to send to the extension.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Drop for NextTicket {
    fn drop(&mut self) {
        self.shared.tickets.release(&self.value);
    }
}

/// The future an extension socket's handler returns.
pub type ExtSocketFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
/// An extension socket's handler.
pub type ExtSocketHandler = Arc<dyn Fn(ExtSocket) -> ExtSocketFuture + Send + Sync>;

/// A registered extension socket route.
pub(crate) struct ExtSocketRoute {
    pub(crate) path: String,
    pub(crate) handler: ExtSocketHandler,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: u64) -> ConnectionInfo {
        ConnectionInfo {
            connection_id: id,
            session: "s".repeat(32),
            identity: None,
        }
    }

    #[test]
    fn a_render_ticket_is_stable_until_used_then_single_use() {
        let t = Tickets::default();
        let a = t.for_render("/x", "voice", &info(1));
        assert!(a.starts_with(TICKET_PREFIX) && a.len() == TICKET_PREFIX.len() + 32);
        assert_eq!(t.for_render("/x", "voice", &info(1)), a);
        assert_ne!(t.for_render("/x", "voice", &info(2)), a);
        assert_ne!(t.for_render("/x", "other", &info(1)), a);
        assert!(t.redeem(&a, "/y").is_none(), "another path");
        assert!(t.redeem(&a, "/x").is_none(), "burnt by the wrong path");
        let b = t.for_render("/x", "voice", &info(1));
        assert_ne!(b, a);
        let claim = t.redeem(&b, "/x").expect("valid once");
        assert_eq!(
            (claim.label.as_str(), claim.connection.connection_id),
            ("voice", 1)
        );
        assert!(t.redeem(&b, "/x").is_none(), "replayed");
        assert!(t.redeem("rwt_nope", "/x").is_none());
    }

    #[test]
    fn a_held_ticket_lives_with_its_socket_then_expires() {
        let t = Tickets::default();
        let claim = TicketClaim {
            path: "/x".into(),
            label: "voice".into(),
            connection: info(1),
        };
        let held = t.held(claim.clone());
        assert!(t.map.lock().unwrap()[&held].expires.is_none());
        t.release(&held);
        assert!(t.map.lock().unwrap()[&held].expires.is_some());
        let stale = t.held(claim);
        t.map.lock().unwrap().get_mut(&stale).unwrap().expires = Some(Instant::now());
        assert!(t.redeem(&stale, "/x").is_none(), "expired");
        assert!(t.redeem(&held, "/x").is_some());
    }

    #[test]
    fn a_claim_needs_the_same_session_and_identity() {
        let mut c = TicketClaim {
            path: "/x".into(),
            label: "voice".into(),
            connection: info(1),
        };
        let s = "s".repeat(32);
        assert!(claim_matches(&c, Some(&s), None));
        assert!(!claim_matches(&c, Some("other"), None));
        assert!(!claim_matches(&c, None, None));
        assert!(!claim_matches(&c, Some(&s), Some(&Identity::new("eve"))));
        c.connection.identity = Some(Identity::new("ann"));
        assert!(claim_matches(&c, Some(&s), Some(&Identity::new("ann"))));
        assert!(!claim_matches(&c, Some(&s), Some(&Identity::new("eve"))));
        assert!(!claim_matches(&c, Some(&s), None));
    }

    #[test]
    fn the_ticket_is_read_from_the_query() {
        assert_eq!(ticket_of("/_rw/ext/audio/ws?t=rwt_ab"), Some("rwt_ab"));
        assert_eq!(ticket_of("/p?a=1&t=rwt_cd"), Some("rwt_cd"));
        assert_eq!(ticket_of("/p"), None);
        assert_eq!(ticket_of("/p?x=1"), None);
    }

    #[tokio::test]
    async fn a_render_outside_a_connection_has_no_ticket() {
        assert!(ticket("/x", "voice").is_none());
        assert!(current_connection().is_none());
        let shared = SharedServerState::new(Duration::from_millis(100));
        let got = with_scope(async move {
            enter(info(7), shared);
            (
                current_connection().map(|c| c.connection_id),
                ticket("/x", "v"),
            )
        })
        .await;
        assert_eq!(got.0, Some(7));
        assert!(got.1.is_some());
    }
}
