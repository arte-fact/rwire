//! Canvas game server.
//!
//! Wraps a TCP listener with WebSocket upgrade, serving the canvas capsule
//! on HTTP GET and running a per-connection game loop on WebSocket connections.

use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::capsule::generate_canvas_capsule;
use crate::game_loop::{GameLoop, SceneLoop};
use crate::input::InputState;
use crate::protocol::encoder::CanvasBuffer;
use crate::scene::{ClientView, Scene};
use crate::scene_diff;

/// Canvas game server builder.
pub struct CanvasServer {
    addr: SocketAddr,
    canvas_width: u16,
    canvas_height: u16,
    tick_rate: u32,
    title: String,
    static_dir: Option<PathBuf>,
}

impl CanvasServer {
    /// Bind to an address.
    pub fn bind(addr: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            addr: addr.parse()?,
            canvas_width: 960,
            canvas_height: 640,
            tick_rate: 20,
            title: "rwire Canvas".to_string(),
            static_dir: None,
        })
    }

    /// Set the canvas dimensions.
    pub fn canvas_size(mut self, width: u16, height: u16) -> Self {
        self.canvas_width = width;
        self.canvas_height = height;
        self
    }

    /// Set the game tick rate in Hz (default 20).
    pub fn tick_rate(mut self, hz: u32) -> Self {
        self.tick_rate = hz;
        self
    }

    /// Set the page title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set a directory for serving static files (sprites, etc.).
    pub fn static_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(path.into());
        self
    }

    /// Run the server with the given game loop (immediate mode).
    pub async fn run<G: GameLoop>(self, game: G) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.addr).await?;
        let game = Arc::new(game);
        let capsule = Arc::new(generate_canvas_capsule(
            self.canvas_width, self.canvas_height, &self.title,
        ));
        let static_dir = self.static_dir.map(Arc::new);
        let tick_ms = (1000 / self.tick_rate.max(1)) as u64;
        println!("Canvas server listening on http://{}", self.addr);
        loop {
            let (stream, peer) = listener.accept().await?;
            let game = Arc::clone(&game);
            let capsule = Arc::clone(&capsule);
            let static_dir = static_dir.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer, game, capsule, static_dir, tick_ms).await {
                    eprintln!("[{peer}] Error: {e}");
                }
            });
        }
    }

    /// Run the server with a retained-mode scene loop.
    pub async fn run_scene<G: SceneLoop>(self, game: G) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.addr).await?;
        let game = Arc::new(game);
        let capsule = Arc::new(generate_canvas_capsule(
            self.canvas_width, self.canvas_height, &self.title,
        ));
        let static_dir = self.static_dir.map(Arc::new);
        let tick_ms = (1000 / self.tick_rate.max(1)) as u64;
        println!("Canvas server listening on http://{}", self.addr);
        loop {
            let (stream, peer) = listener.accept().await?;
            let game = Arc::clone(&game);
            let capsule = Arc::clone(&capsule);
            let static_dir = static_dir.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection_scene(stream, peer, game, capsule, static_dir, tick_ms).await {
                    eprintln!("[{peer}] Error: {e}");
                }
            });
        }
    }
}

async fn handle_connection<G: GameLoop>(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    game: Arc<G>,
    capsule: Arc<String>,
    static_dir: Option<Arc<PathBuf>>,
    tick_ms: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    // Peek at first bytes to determine if HTTP or WebSocket upgrade
    let mut buf = vec![0u8; 4096];
    let stream = stream.into_std()?;
    stream.set_nonblocking(false)?;

    let mut stream = tokio::net::TcpStream::from_std(stream)?;
    let n = stream.peek(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    if request.contains("Upgrade: websocket") || request.contains("upgrade: websocket") {
        // WebSocket upgrade
        let ws = tokio_tungstenite::accept_async(stream).await?;
        println!("[{peer}] WebSocket connected");
        handle_websocket(ws, peer, game, tick_ms).await?;
    } else if request.starts_with("GET ") {
        // HTTP request
        let path = request
            .split(' ')
            .nth(1)
            .unwrap_or("/")
            .split('?')
            .next()
            .unwrap_or("/");

        // Try static file first
        if let Some(ref dir) = static_dir {
            if path.len() > 1 {
                let file_path = dir.join(path.trim_start_matches('/'));
                if file_path.is_file() {
                    let data = tokio::fs::read(&file_path).await?;
                    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                        Some("png") => "image/png",
                        Some("jpg" | "jpeg") => "image/jpeg",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        Some("svg") => "image/svg+xml",
                        Some("js") => "application/javascript",
                        Some("css") => "text/css",
                        Some("json") => "application/json",
                        Some("woff2") => "font/woff2",
                        Some("mp3") => "audio/mpeg",
                        Some("ogg") => "audio/ogg",
                        _ => "application/octet-stream",
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: public, max-age=86400\r\n\r\n",
                        data.len()
                    );
                    stream.write_all(response.as_bytes()).await?;
                    stream.write_all(&data).await?;
                    return Ok(());
                }
            }
        }

        // Serve capsule HTML (no-cache to ensure JS runtime matches server opcodes)
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n{}",
            capsule.len(),
            &*capsule
        );
        stream.write_all(response.as_bytes()).await?;
    }

    Ok(())
}

async fn handle_websocket<G: GameLoop>(
    ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer: SocketAddr,
    game: Arc<G>,
    tick_ms: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (mut ws_write, mut ws_read) = ws.split();

    // Initialize game state
    let mut state = game.init();
    let mut input = InputState::default();

    // Send setup data (texture tables, cached images, etc.)
    let mut setup_buf = CanvasBuffer::new();
    game.setup(&state, &mut setup_buf);
    let setup_bytes = setup_buf.finish();
    if !setup_bytes.is_empty() {
        ws_write.send(Message::Binary(setup_bytes.to_vec())).await?;
    }

    let dt = tick_ms as f32 / 1000.0;
    let mut tick_count = 0u32;
    let mut tick_interval = interval(Duration::from_millis(tick_ms));

    loop {
        tokio::select! {
            // Game tick
            _ = tick_interval.tick() => {
                // Tick game logic
                game.tick(&mut state, &input, dt);

                // Render frame
                let mut buf = CanvasBuffer::with_capacity(2048);
                buf.frame_begin(tick_count);
                game.render(&state, &mut buf);
                buf.frame_end();

                let bytes = buf.finish();
                if ws_write.send(Message::Binary(bytes.to_vec())).await.is_err() {
                    break;
                }

                tick_count += 1;
            }

            // Incoming messages (input)
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        input = InputState::decode(&data);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    println!("[{peer}] Disconnected");
    Ok(())
}

// ── Scene-mode connection handler ──────────────────────────────────

async fn handle_connection_scene<G: SceneLoop>(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    game: Arc<G>,
    capsule: Arc<String>,
    static_dir: Option<Arc<PathBuf>>,
    tick_ms: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    let mut buf = vec![0u8; 4096];
    let stream = stream.into_std()?;
    stream.set_nonblocking(false)?;
    let mut stream = tokio::net::TcpStream::from_std(stream)?;
    let n = stream.peek(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    if request.contains("Upgrade: websocket") || request.contains("upgrade: websocket") {
        let ws = tokio_tungstenite::accept_async(stream).await?;
        println!("[{peer}] WebSocket connected (scene mode)");
        handle_websocket_scene(ws, peer, game, tick_ms).await?;
    } else if request.starts_with("GET ") {
        let path = request.split(' ').nth(1).unwrap_or("/").split('?').next().unwrap_or("/");
        if let Some(ref dir) = static_dir {
            if path.len() > 1 {
                let file_path = dir.join(path.trim_start_matches('/'));
                if file_path.is_file() {
                    let data = tokio::fs::read(&file_path).await?;
                    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                        Some("png") => "image/png",
                        Some("jpg" | "jpeg") => "image/jpeg",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        Some("svg") => "image/svg+xml",
                        Some("js") => "application/javascript",
                        Some("css") => "text/css",
                        Some("json") => "application/json",
                        Some("woff2") => "font/woff2",
                        Some("mp3") => "audio/mpeg",
                        Some("ogg") => "audio/ogg",
                        _ => "application/octet-stream",
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: public, max-age=86400\r\n\r\n",
                        data.len()
                    );
                    stream.write_all(response.as_bytes()).await?;
                    stream.write_all(&data).await?;
                    return Ok(());
                }
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            capsule.len(), &*capsule
        );
        stream.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

async fn handle_websocket_scene<G: SceneLoop>(
    ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer: SocketAddr,
    game: Arc<G>,
    tick_ms: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (mut ws_write, mut ws_read) = ws.split();

    let mut state = game.init();
    let mut input = InputState::default();
    let mut scene = Scene::new();
    let mut client_view = ClientView::new();

    // Setup: send tick interval, layers, tilemaps, minimap, initial sprites
    let mut setup_buf = CanvasBuffer::new();
    setup_buf.tick_interval(tick_ms as u16);
    game.setup_scene(&mut state, &mut scene, &mut setup_buf);

    // Emit layer creation opcodes
    scene_diff::diff_layers(&scene, &mut client_view, &mut setup_buf);

    // Initial scene population — diff against empty client view sends all as creates
    scene_diff::diff_sprites(&scene, &client_view, &mut setup_buf);
    client_view.sync_from(&scene);

    let setup_bytes = setup_buf.finish();
    if !setup_bytes.is_empty() {
        ws_write.send(Message::Binary(setup_bytes.to_vec())).await?;
    }

    let dt = tick_ms as f32 / 1000.0;
    let mut tick_count = 0u32;
    let mut tick_interval = interval(Duration::from_millis(tick_ms));

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                game.tick(&mut state, &input, dt);
                game.update_scene(&state, &mut scene);

                let mut buf = CanvasBuffer::with_capacity(1024);

                // Scene tick marker (client snapshots prev positions)
                buf.scene_tick(tick_count);

                // Camera
                let (cx, cy, zoom) = game.camera(&state);
                let cam_x = cx.round() as i16;
                let cam_y = cy.round() as i16;
                let cam_zoom = (zoom * 256.0).round() as u16;
                if scene_diff::diff_camera(cam_x, cam_y, cam_zoom, &client_view, &mut buf) {
                    client_view.last_camera = (cam_x, cam_y, cam_zoom);
                }

                // Sprite diffs
                scene_diff::diff_sprites(&scene, &client_view, &mut buf);
                client_view.sync_from(&scene);

                // Scene end marker
                buf.scene_end();

                // Overlay (HUD, immediate-mode — sent as-is)
                game.render_overlay(&state, &mut buf);

                let bytes = buf.finish();
                if ws_write.send(Message::Binary(bytes.to_vec())).await.is_err() {
                    break;
                }
                tick_count += 1;
            }

            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        input = InputState::decode(&data);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    println!("[{peer}] Disconnected");
    Ok(())
}
