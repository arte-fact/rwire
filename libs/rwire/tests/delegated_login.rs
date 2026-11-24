//! Login form for delegated authentication, over a real socket: rwire serves the
//! app's page, the app judges the submitted fields, rwire stores the session
//! value, and the same cookie opens the page and the WebSocket.
//!
//! The authority is a closure here (no hub): what matters is the HTTP/WS
//! contract, not who answers.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::delegated_auth::{DelegatedAuth, Identity, LoginForm};
use rwire::{el, El, ElementBuilder, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const ADDR: &str = "127.0.0.1:19351";
const COOKIE: &str = "session";
const SECRET: &str = "s3cret";

/// The session values the app was told to retire, in order.
static RETIRED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn page() -> ElementBuilder {
    el(El::Div).text("contenu protégé")
}

/// The app's login page: the error message, the form, the password field. Drawn
/// with tokens, like any app page.
fn login_page(error: Option<&str>) -> ElementBuilder {
    let mut kids = vec![
        el(El::H1).text("Connexion"),
        el(El::Form)
            .attr("method", "POST")
            .attr("action", "/login")
            .append([
                el(El::Input)
                    .attr("type", "password")
                    .attr("name", "password")
                    .attr("aria-label", "Mot de passe"),
                el(El::Button).attr("type", "submit").text("Entrer"),
            ]),
    ];
    if let Some(message) = error {
        kids.push(el(El::P).text(message));
    }
    el(El::Div).append(kids)
}

/// The authority: the password must match, and the session value it mints is
/// what rwire stores (here: a fixed token, to assert on it).
fn auth() -> DelegatedAuth {
    DelegatedAuth::new(COOKIE, |request| async move {
        let ok = request.cookie(COOKIE) == Some("jeton");
        ok.then(|| Identity::new("admin").with_data("jeton".to_string()))
    })
    .login_form(|form: LoginForm| async move {
        match form.field("password") {
            Some(p) if p == SECRET => Ok("jeton".to_string()),
            _ => Err("Mot de passe refusé".to_string()),
        }
    })
    .login_page(login_page)
    .on_logout(|value| async move {
        RETIRED.lock().unwrap().push(value);
    })
}

async fn start() {
    let server = Server::bind(ADDR).unwrap().root(page).auth_with(auth());
    tokio::spawn(async move {
        let _ = server.run().await;
    });
    for _ in 0..100 {
        if TcpStream::connect(ADDR).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the server never started");
}

/// Send a raw request, return the whole response.
async fn send(request: &str) -> String {
    let mut stream = TcpStream::connect(ADDR).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
    let mut response = Vec::new();
    let read =
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response)).await;
    assert!(matches!(read, Ok(Ok(_))), "no answer: {read:?}");
    String::from_utf8_lossy(&response).into_owned()
}

fn get(path: &str, cookie: Option<&str>) -> String {
    let cookie = cookie.map_or(String::new(), |c| format!("Cookie: {COOKIE}={c}\r\n"));
    format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{cookie}Connection: close\r\n\r\n")
}

fn post_login(body: &str) -> String {
    format!(
        "POST /login HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_login_form_stores_the_authority_token_and_logout_clears_it() {
    start().await;

    // 1. Without a cookie, the app's page is shown (200), not a redirect.
    let anon = send(&get("/", None)).await;
    assert!(anon.starts_with("HTTP/1.1 200"), "{anon}");
    assert!(
        anon.contains("Connexion"),
        "the login page must be served: {anon}"
    );
    assert!(anon.contains("name=\"password\""), "{anon}");

    // 2. A refused attempt re-renders the page with the app's message.
    let refused = send(&post_login("password=wrong")).await;
    assert!(refused.starts_with("HTTP/1.1 200"), "{refused}");
    assert!(refused.contains("Mot de passe refusé"), "{refused}");
    assert!(!refused.contains("Set-Cookie"), "{refused}");

    // 3. A good attempt sets the offered cookie value and redirects.
    let ok = send(&post_login("password=s3cret")).await;
    assert!(ok.starts_with("HTTP/1.1 303"), "{ok}");
    assert!(
        ok.contains(&format!("Set-Cookie: {COOKIE}=jeton;")),
        "the authority's value must be stored: {ok}"
    );
    assert!(ok.contains("Location: /"), "{ok}");

    // `next` is honoured when it is a path on this app, refused otherwise.
    let next = send(&post_login("password=s3cret&next=%2Fmachines")).await;
    assert!(next.contains("Location: /machines"), "{next}");
    let evil = send(&post_login("password=s3cret&next=%2F%2Fevil.example")).await;
    assert!(
        evil.contains("Location: /"),
        "next must stay on this app: {evil}"
    );

    // 4. With the cookie, the page is served — the app shell, not the form. (The
    // content itself arrives over the WebSocket: the capsule is the runtime.)
    let authed = send(&get("/", Some("jeton"))).await;
    assert!(authed.starts_with("HTTP/1.1 200"), "{authed}");
    assert!(
        !authed.contains("name=\"password\""),
        "the login form must not be served to a logged-in visitor: {authed}"
    );
    assert!(
        authed.contains("RWV="),
        "the app shell must be served: {authed}"
    );

    // 5. The same cookie opens the WebSocket.
    let mut request = format!("ws://{ADDR}/").into_client_request().unwrap();
    request
        .headers_mut()
        .insert("cookie", format!("{COOKIE}=jeton").parse().unwrap());
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    ws.send(Message::Text("R/".to_string())).await.unwrap();
    let frame = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Binary(data) = message {
                return Some(String::from_utf8_lossy(&data).into_owned());
            }
        }
        None
    })
    .await
    .ok()
    .flatten();
    let frame = frame.expect("no frame after the route");
    assert!(frame.contains("contenu protégé"), "{frame}");

    // 6. Without the cookie, the handshake is refused (401), as before.
    let mut anon_ws = format!("ws://{ADDR}/").into_client_request().unwrap();
    anon_ws
        .headers_mut()
        .insert("cookie", "session=faux".parse().unwrap());
    let refused_ws = tokio_tungstenite::connect_async(anon_ws).await;
    assert!(refused_ws.is_err(), "a refused session must not upgrade");

    // 7. Logout clears the cookie and sends back to the form.
    let out = send(&format!(
        "POST /logout HTTP/1.1\r\nHost: localhost\r\nCookie: {COOKIE}=jeton\r\n\
         Content-Length: 0\r\nConnection: close\r\n\r\n"
    ))
    .await;
    assert!(out.starts_with("HTTP/1.1 303"), "{out}");
    assert!(out.contains("Location: /login"), "{out}");
    assert!(
        out.contains(&format!("Set-Cookie: {COOKIE}=;")) && out.contains("Max-Age=0"),
        "the cookie must be expired: {out}"
    );

    // 8. The app was told which session to retire, before the cookie went.
    let retired = RETIRED.lock().unwrap().clone();
    assert_eq!(
        retired,
        vec!["jeton".to_string()],
        "the logout hook must receive the session value once"
    );

    // A visitor without the cookie retires nothing.
    let anonymous =
        send("GET /logout HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await;
    assert!(anonymous.starts_with("HTTP/1.1 303"), "{anonymous}");
    assert_eq!(
        RETIRED.lock().unwrap().len(),
        1,
        "an anonymous logout retires no session"
    );
}
