//! A tokio application hosting rwire: open http://127.0.0.1:9009.
//!
//! The snapshot line follows a simulated hub stream produced on tokio; "Ask tokio"
//! runs work on the tokio runtime and writes the answer back. Ctrl-C stops both
//! sides. See the crate docs (`src/lib.rs`) for the pattern and its pitfalls.

use std::time::Duration;

use tokio::runtime::Handle;
use tokio_bridge::{simulate_hub, start};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("tokio-bridge on http://127.0.0.1:9009");
    let (rwire, snapshots) = start("127.0.0.1:9009", Handle::current())?;
    let producer = tokio::spawn(simulate_hub(snapshots, Duration::from_secs(1)));

    tokio::signal::ctrl_c().await?;
    println!("stopping");
    // Ending the producer drops the last snapshot sender, which ends the rwire-side
    // applier; then the rwire thread is stopped and joined off the tokio workers.
    producer.abort();
    tokio::task::spawn_blocking(move || rwire.shutdown()).await??;
    Ok(())
}
