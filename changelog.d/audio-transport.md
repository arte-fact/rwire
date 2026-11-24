### Added: extension sockets and the `rwire-audio` transport

- **`rwire::ext_socket`**: a WebSocket route of a runtime extension's own, `ServerWithRoot::ext_socket(path, handler)`.
  - The handshake passes the page socket's gates (auth, Origin, admission caps) and must carry a single-use ticket (`?t=`).
  - A render issues the ticket with `ext_socket::ticket(path, label)`. It is bound to the page connection: session cookie, delegated identity, connection id.
  - A ticket that is unknown, replayed, expired, for another path, or presented with another session or identity closes the socket with `4403`.
  - `ExtSocket::next_ticket()` hands the extension the ticket of its next connection, so a lost socket reconnects with no render.
  - `ext_socket::current_connection()` tells a render which page connection it runs for.
- **New crate `rwire-audio`**, transport only so far:
  - `.serve_audio(|link: AudioLink| async move { … })` serves `/_rw/ext/audio/ws`;
  - `el.audio(Audio::new(name))` renders the stream and its ticket;
  - `AudioLink` carries `session()`, `name()`, `connection_id()`, `identity()` and `server()` (the `SharedServerState`, to update a state and re-render);
  - `link.input.recv()` yields 20 ms `Frame`s (`seq`, `timestamp`, 320 PCM16 samples at 16 kHz) from a bounded queue (1 s, oldest dropped first, the gap shows in `seq`);
  - `link.output` offers `pcm(samples, rate)`, `mark(id)`, `sound(name)`, `stop()` and `end()`.
