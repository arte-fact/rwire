//! Web Push, minimal: a button turns notifications on, another sends one to
//! every subscriber.
//!
//! ```text
//! cargo run -p push      # then open http://127.0.0.1:9010
//! ```
//!
//! What the app writes: the buttons (`push_subscribe`, `push_unsubscribe`),
//! the hidden `report_input` and its handler, and the moment to send (here a
//! button; in a blog, publishing a post). What `rwire-push` provides: the
//! VAPID key (`push-data/vapid.key`, created on the first run, or the
//! `PUSH_VAPID_KEY` environment variable), the subscriptions
//! (`push-data/subscriptions.json`), the encryption, the signed request to
//! each browser's push service, and the purge of the dead subscriptions.
//!
//! Push needs a secure context: `http://127.0.0.1` or `localhost` counts as
//! one in Chrome, Edge and Firefox; anything else needs https. On iOS and
//! iPadOS (16.4+), only once the app is added to the home screen.

use std::error::Error;
use std::sync::OnceLock;

use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{handler, renderer, theme, ElementBuilder, Pwa, Server, St, State};
use rwire_components::{Button, Card, Gap, Stack, Text};
use rwire_push::{report_input, Notification, Push, PushExt, PushState, ServePush, VapidKey};
use rwire_themes::palettes;

/// The push service, set once in `main` (handlers take only their state).
static PUSH: OnceLock<Push> = OnceLock::new();

fn push() -> &'static Push {
    PUSH.get().expect("PUSH is set in main")
}

/// Per-connection: what this browser reported.
#[derive(State, Default)]
#[storage(memory)]
struct Me {
    push: PushState,
}

/// The extension reported (on load, then after each click): store or
/// forget the subscription, and remember what to show.
#[handler]
fn on_push_report(me: &mut Me, ctx: &EventContext) {
    me.push = push().handle_report(ctx.text().unwrap_or(""));
}

/// Send a notification to every subscriber. The button spins until the
/// push services have answered.
#[handler]
fn send_test(_me: &mut Me, ctx: &EventContext) {
    let done = ctx.defer();
    tokio::spawn(async move {
        let n = Notification::new("Hello from rwire")
            .body("A test notification. Click to open the app.")
            .url("/");
        match push().send_push(&n).await {
            Ok(r) if r.failed.is_empty() => done.ok(),
            Ok(r) => done.err(format!("{} failed: {}", r.failed.len(), r.failed[0].1)),
            Err(e) => done.err(e.to_string()),
        }
    });
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let vapid = match VapidKey::from_env("PUSH_VAPID_KEY") {
        Ok(k) => k,
        Err(_) => VapidKey::load_or_generate("push-data/vapid.key")?,
    };
    let p = Push::new(vapid, "mailto:admin@example.org").store("push-data/subscriptions.json")?;
    println!(
        "push on http://127.0.0.1:9010 ({} subscriptions)",
        p.subscriptions().len()
    );
    PUSH.set(p).expect("set once");

    Server::bind("127.0.0.1:9010")?
        .root(app)
        .capsule_config(CapsuleConfig::new().pwa(Pwa::new("rwire push").push_notifications(true)))
        .theme(app_theme())
        .serve_push()
        .run()
        .await
}

fn app() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .align_center()
        .children([
            Card::new()
                .child(
                    Stack::column()
                        .gap(Gap::Md)
                        .children([
                            Text::heading2("Push notifications").build(),
                            render_status(),
                            Button::secondary("Send a test notification").on_click(send_test()),
                        ])
                        .build(),
                )
                .build(),
            // Outside the region: one report input for the page's lifetime.
            report_input("push-report", on_push_report()),
        ])
        .build()
        .st([St::BgApp, St::MinHDvh, St::PLg])
}

#[renderer]
fn render_status(me: &Me) -> ElementBuilder {
    let (text, button) = match &me.push {
        PushState::Subscribed => (
            "Notifications are on for this browser.".to_string(),
            Some(Button::outline("Turn off").build().push_unsubscribe()),
        ),
        PushState::Denied => (
            "Notifications are blocked: allow them in the browser's site settings.".to_string(),
            None,
        ),
        PushState::Unsupported => (
            "This browser has no Web Push here. On iPhone and iPad, add the app to the home \
             screen first."
                .to_string(),
            None,
        ),
        PushState::Failed(why) => (
            format!("Could not turn notifications on: {why}"),
            Some(Button::primary("Try again").build().push_subscribe(push())),
        ),
        PushState::Unknown | PushState::Unsubscribed => (
            "Notifications are off.".to_string(),
            Some(
                Button::primary("Turn on notifications")
                    .build()
                    .push_subscribe(push()),
            ),
        ),
    };
    let mut children = vec![Text::body(text).muted().build()];
    children.extend(button);
    Stack::column().gap(Gap::Sm).children(children).build()
}
