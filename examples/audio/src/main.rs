//! Microphone and playback with `rwire-audio`: speak with the microphone
//! on, turn it off, and the server plays back what it heard (after a
//! "ding"). "Play a tone" plays two seconds of 440 Hz with a mark every
//! half second; "Stop" cuts it.
//!
//! ```text
//! cargo run -p audio     # then open http://127.0.0.1:9012
//! ```
//!
//! The page also shows what the browser reports (capture and output state,
//! playback events on the audio clock) and two measurements the end-to-end
//! check (`runtime/e2e/audio-chrome.mjs`) reads:
//! - `playmax`: the loudest microphone frame heard while the tone played
//!   (echo: it should stay low);
//! - `stopms`: from the stop order to the silence, on the audio clock.
//!
//! The microphone needs a secure context: `http://127.0.0.1` or `localhost`
//! is one; anything else needs https.

use std::error::Error;
use std::sync::Mutex;

use rwire::theme::Theme;
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Server, St, State};
use rwire_audio::{
    parse_event, Audio, AudioEvent, AudioExt, AudioLink, Frame, OutputError, ServeAudio,
};
use rwire_components::{Button, Gap, Stack};
use rwire_themes::palettes;
use tokio::sync::mpsc;

/// What the page asks the audio task to do.
enum Cmd {
    Replay,
    Tone,
    Stop,
}

/// The current audio task's inbox (the latest link wins: one page).
static INBOX: Mutex<Option<mpsc::UnboundedSender<Cmd>>> = Mutex::new(None);

fn send(cmd: Cmd) {
    if let Some(tx) = INBOX.lock().unwrap().as_ref() {
        let _ = tx.send(cmd);
    }
}

/// Per connection: whether the microphone is on.
#[derive(State, Default)]
#[storage(memory)]
struct Me {
    listening: bool,
}

/// What the browser and the audio task report, for every page.
#[derive(State, Default)]
#[storage(shared)]
struct Studio {
    capture: String,
    output: String,
    events: Vec<String>,
    frames: u64,
    level: f32,
    /// Frame levels of the last ten seconds: (audio-clock time, level).
    levels: Vec<(f64, f32)>,
    /// The last playback window, from `PlaybackStarted` to its end or stop.
    playing: Option<(f64, Option<f64>)>,
    /// The audio clock when "Stop" was ordered (the last frame's end).
    stop_ordered: Option<f64>,
    stop_ms: Option<f64>,
}

impl Studio {
    fn log(&mut self, line: String) {
        self.events.push(line);
        let n = self.events.len();
        if n > 12 {
            self.events.drain(..n - 12);
        }
    }

    /// The loudest frame captured during the last playback window.
    fn play_max(&self) -> Option<f32> {
        let (from, to) = self.playing?;
        let to = to?;
        self.levels
            .iter()
            .filter(|(t, _)| *t >= from && *t <= to)
            .map(|(_, l)| *l)
            .reduce(f32::max)
    }
}

#[handler]
fn toggle_mic(me: &mut Me) {
    me.listening = !me.listening;
    if !me.listening {
        send(Cmd::Replay);
    }
}

#[handler]
fn play_tone(_me: &mut Me) {
    send(Cmd::Tone);
}

#[handler]
fn stop_now(_me: &mut Me) {
    send(Cmd::Stop);
}

/// What the browser says about the stream.
#[handler]
fn on_audio(studio: &mut Studio, ctx: &rwire::EventContext) {
    let Some(event) = parse_event(ctx) else {
        return;
    };
    match &event {
        AudioEvent::Capture(s) => studio.capture = format!("{s:?}"),
        AudioEvent::Output(s) => studio.output = format!("{s:?}"),
        AudioEvent::PlaybackStarted { at } => studio.playing = Some((*at, None)),
        AudioEvent::PlaybackEnded { at } => {
            if let Some(p) = studio.playing.as_mut() {
                p.1 = Some(*at);
            }
        }
        AudioEvent::Stopped { at, .. } => {
            if let Some(p) = studio.playing.as_mut() {
                p.1 = Some(*at);
            }
            if let Some(ordered) = studio.stop_ordered.take() {
                studio.stop_ms = Some((at - ordered) * 1000.0);
            }
        }
        _ => {}
    }
    studio.log(format!("{event:?}"));
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

/// The RMS level of a frame, 0..1.
fn level(f: &Frame) -> f32 {
    let sum: f64 = f.samples.iter().map(|&s| f64::from(s).powi(2)).sum();
    ((sum / f.samples.len() as f64).sqrt() / 32768.0) as f32
}

/// Two seconds of 440 Hz at 24 kHz, half a second at a time, each followed
/// by a mark. Refused (a full queue, a closed link): reported, not played.
fn tone(link: &AudioLink) -> Result<(), OutputError> {
    const RATE: u32 = 24_000;
    for part in 0..4 {
        let pcm: Vec<i16> = (0..RATE / 2)
            .map(|k| {
                let t = f64::from(part * RATE / 2 + k) / f64::from(RATE);
                ((t * 440.0 * std::f64::consts::TAU).sin() * 16_000.0) as i16
            })
            .collect();
        link.output.pcm(&pcm, RATE)?;
        link.output.mark(&format!("t{}", part + 1))?;
    }
    link.output.end()
}

/// Play back what was heard, after the ding.
fn replay(link: &AudioLink, pcm: &[i16]) -> Result<(), OutputError> {
    link.output.sound("ding")?;
    link.output.pcm(pcm, 16_000)?;
    link.output.mark("echo")?;
    link.output.end()
}

/// A 150 ms "ding" at 880 Hz, as a WAV file.
fn ding_wav() -> Vec<u8> {
    const RATE: u32 = 24_000;
    let n = RATE * 15 / 100;
    let mut out = Vec::new();
    let data_len = n * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for k in 0..n {
        let t = f64::from(k) / f64::from(RATE);
        let fade = 1.0 - f64::from(k) / f64::from(n);
        let s = ((t * 880.0 * std::f64::consts::TAU).sin() * 12_000.0 * fade) as i16;
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// One connection of the page's audio stream.
async fn on_link(mut link: AudioLink) {
    let (tx, mut inbox) = mpsc::unbounded_channel();
    *INBOX.lock().unwrap() = Some(tx);
    let mut heard: Vec<i16> = Vec::new();
    let mut last_end = 0.0;
    let mut batch: Vec<(f64, f32)> = Vec::new();
    loop {
        tokio::select! {
            frame = link.input.recv() => {
                let Some(frame) = frame else { break };
                let l = level(&frame);
                last_end = frame.timestamp + 0.02;
                batch.push((frame.timestamp, l));
                if heard.len() < 16_000 * 30 {
                    heard.extend_from_slice(&frame.samples);
                }
                // The page is updated five times a second, not fifty.
                if batch.len() == 10 {
                    let batch = std::mem::take(&mut batch);
                    link.server().update_shared::<Studio>(move |s| {
                        s.frames += batch.len() as u64;
                        s.level = batch.last().map_or(0.0, |b| b.1);
                        s.levels.extend(batch);
                        let horizon = last_end - 10.0;
                        s.levels.retain(|(t, _)| *t >= horizon);
                    });
                }
            }
            cmd = inbox.recv() => match cmd {
                Some(Cmd::Replay) => {
                    let pcm = std::mem::take(&mut heard);
                    if let Err(e) = replay(&link, &pcm) {
                        eprintln!("replay refused: {e}");
                    }
                }
                Some(Cmd::Tone) => {
                    if let Err(e) = tone(&link) {
                        eprintln!("tone refused: {e}");
                    }
                }
                Some(Cmd::Stop) => {
                    link.output.stop();
                    link.server().update_shared::<Studio>(move |s| s.stop_ordered = Some(last_end));
                }
                None => break,
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("audio on http://127.0.0.1:9012");
    Server::bind("127.0.0.1:9012")?
        .root(app)
        .theme(app_theme())
        .asset("/sounds/ding.wav", "audio/wav", ding_wav())
        .serve_audio(on_link)
        .run()
        .await
}

fn app() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([controls(), studio()])
        .build()
        .st([St::BgApp, St::MinHDvh, St::PLg])
}

#[renderer]
fn controls(me: &Me) -> ElementBuilder {
    Stack::row()
        .gap(Gap::Md)
        .children([
            Button::primary(if me.listening {
                "Microphone off (play it back)"
            } else {
                "Microphone on"
            })
            .on_click(toggle_mic()),
            Button::secondary("Play a tone").on_click(play_tone()),
            Button::secondary("Stop").on_click(stop_now()),
            el(El::Div).attr("id", "stream").audio(
                Audio::new("voice")
                    .capture(me.listening)
                    .sounds([("ding", "/sounds/ding.wav")])
                    .on_event(on_audio()),
            ),
        ])
        .build()
}

#[renderer]
fn studio(s: &Studio) -> ElementBuilder {
    let measure = format!(
        "capture={} output={} frames={} level={:.3} playmax={} stopms={}",
        s.capture,
        s.output,
        s.frames,
        s.level,
        s.play_max().map_or("-".into(), |m| format!("{m:.4}")),
        s.stop_ms.map_or("-".into(), |m| format!("{m:.0}")),
    );
    Stack::column()
        .gap(Gap::Sm)
        .children(
            std::iter::once(el(El::P).attr("id", "measure").text(&measure)).chain(
                s.events
                    .iter()
                    .map(|e| el(El::P).st([St::TextMuted]).text(e)),
            ),
        )
        .build()
}
