//! Prometheus-compatible metrics for rwire servers.
//!
//! Provides counters, gauges, and histograms for monitoring server health.
//!
//! # Example
//!
//! ```ignore
//! use rwire::metrics::Metrics;
//!
//! let metrics = Metrics::new();
//! metrics.connections_total.inc();
//! metrics.active_connections.set(42);
//!
//! // Export as Prometheus format
//! println!("{}", metrics.to_prometheus());
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// A simple counter metric.
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    /// Create a new counter starting at 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by n.
    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Get the current value.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// A gauge metric that can go up or down.
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    /// Create a new gauge starting at 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the gauge to a specific value.
    pub fn set(&self, val: u64) {
        self.value.store(val, Ordering::Relaxed);
    }

    /// Increment the gauge by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the gauge by 1.
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get the current value.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram buckets for latency measurements.
const LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// A histogram metric for measuring distributions.
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<(f64, AtomicU64)>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    /// Create a new histogram with default latency buckets.
    pub fn new() -> Self {
        Self::with_buckets(LATENCY_BUCKETS)
    }

    /// Create a histogram with custom bucket boundaries.
    pub fn with_buckets(boundaries: &[f64]) -> Self {
        let buckets = boundaries.iter().map(|&b| (b, AtomicU64::new(0))).collect();

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Observe a value.
    pub fn observe(&self, val: f64) {
        // Update sum (stored as bits)
        let bits = val.to_bits();
        self.sum.fetch_add(bits, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Increment appropriate bucket
        for (boundary, counter) in &self.buckets {
            if val <= *boundary {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get the total count of observations.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Start a timer that records duration on drop.
    pub fn start_timer(&self) -> HistogramTimer<'_> {
        HistogramTimer {
            histogram: self,
            start: Instant::now(),
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for measuring duration and recording to a histogram.
pub struct HistogramTimer<'a> {
    histogram: &'a Histogram,
    start: Instant,
}

impl<'a> Drop for HistogramTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

/// Collection of all server metrics.
pub struct Metrics {
    /// Total connections since server start.
    pub connections_total: Counter,
    /// Currently active connections.
    pub active_connections: Gauge,
    /// Total WebSocket messages received.
    pub messages_received: Counter,
    /// Total WebSocket messages sent.
    pub messages_sent: Counter,
    /// Total bytes received.
    pub bytes_received: Counter,
    /// Total bytes sent.
    pub bytes_sent: Counter,
    /// Handler execution latency.
    pub handler_latency: Histogram,
    /// WebSocket message processing latency.
    pub message_latency: Histogram,
    /// Connection errors.
    pub connection_errors: Counter,
    /// Handler errors.
    pub handler_errors: Counter,
    /// Connections rejected due to limits.
    pub connections_rejected: Counter,
    /// Server start time (Unix timestamp).
    start_time: u64,
    /// Custom labels for this server instance.
    labels: RwLock<Vec<(String, String)>>,
}

impl Metrics {
    /// Create a new metrics collection.
    pub fn new() -> Self {
        Self {
            connections_total: Counter::new(),
            active_connections: Gauge::new(),
            messages_received: Counter::new(),
            messages_sent: Counter::new(),
            bytes_received: Counter::new(),
            bytes_sent: Counter::new(),
            handler_latency: Histogram::new(),
            message_latency: Histogram::new(),
            connection_errors: Counter::new(),
            handler_errors: Counter::new(),
            connections_rejected: Counter::new(),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            labels: RwLock::new(Vec::new()),
        }
    }

    /// Add a label to all metrics.
    pub fn add_label(&self, key: &str, value: &str) {
        self.labels
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push((key.to_string(), value.to_string()));
    }

    /// Get labels as a string for Prometheus format.
    fn labels_str(&self) -> String {
        let labels = self.labels.read().unwrap_or_else(|e| e.into_inner());
        if labels.is_empty() {
            return String::new();
        }
        let pairs: Vec<String> = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect();
        format!("{{{}}}", pairs.join(","))
    }

    /// Export metrics in Prometheus text format.
    pub fn to_prometheus(&self) -> String {
        let labels = self.labels_str();

        let mut output = String::new();

        // Server info
        output.push_str(&format!(
            "# HELP rwire_start_time_seconds Unix timestamp of server start\n\
             # TYPE rwire_start_time_seconds gauge\n\
             rwire_start_time_seconds{} {}\n\n",
            labels, self.start_time
        ));

        // Connection metrics
        output.push_str(&format!(
            "# HELP rwire_connections_total Total connections since start\n\
             # TYPE rwire_connections_total counter\n\
             rwire_connections_total{} {}\n\n",
            labels,
            self.connections_total.get()
        ));

        output.push_str(&format!(
            "# HELP rwire_active_connections Currently active connections\n\
             # TYPE rwire_active_connections gauge\n\
             rwire_active_connections{} {}\n\n",
            labels,
            self.active_connections.get()
        ));

        output.push_str(&format!(
            "# HELP rwire_connections_rejected_total Connections rejected due to limits\n\
             # TYPE rwire_connections_rejected_total counter\n\
             rwire_connections_rejected_total{} {}\n\n",
            labels,
            self.connections_rejected.get()
        ));

        // Message metrics
        output.push_str(&format!(
            "# HELP rwire_messages_received_total WebSocket messages received\n\
             # TYPE rwire_messages_received_total counter\n\
             rwire_messages_received_total{} {}\n\n",
            labels,
            self.messages_received.get()
        ));

        output.push_str(&format!(
            "# HELP rwire_messages_sent_total WebSocket messages sent\n\
             # TYPE rwire_messages_sent_total counter\n\
             rwire_messages_sent_total{} {}\n\n",
            labels,
            self.messages_sent.get()
        ));

        // Byte metrics
        output.push_str(&format!(
            "# HELP rwire_bytes_received_total Bytes received\n\
             # TYPE rwire_bytes_received_total counter\n\
             rwire_bytes_received_total{} {}\n\n",
            labels,
            self.bytes_received.get()
        ));

        output.push_str(&format!(
            "# HELP rwire_bytes_sent_total Bytes sent\n\
             # TYPE rwire_bytes_sent_total counter\n\
             rwire_bytes_sent_total{} {}\n\n",
            labels,
            self.bytes_sent.get()
        ));

        // Error metrics
        output.push_str(&format!(
            "# HELP rwire_connection_errors_total Connection errors\n\
             # TYPE rwire_connection_errors_total counter\n\
             rwire_connection_errors_total{} {}\n\n",
            labels,
            self.connection_errors.get()
        ));

        output.push_str(&format!(
            "# HELP rwire_handler_errors_total Handler execution errors\n\
             # TYPE rwire_handler_errors_total counter\n\
             rwire_handler_errors_total{} {}\n\n",
            labels,
            self.handler_errors.get()
        ));

        // Latency histograms
        output.push_str(&format!(
            "# HELP rwire_handler_latency_seconds Handler execution latency\n\
             # TYPE rwire_handler_latency_seconds histogram\n\
             rwire_handler_latency_seconds_count{} {}\n\n",
            labels,
            self.handler_latency.count()
        ));

        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0);

        gauge.set(10);
        assert_eq!(gauge.get(), 10);

        gauge.inc();
        assert_eq!(gauge.get(), 11);

        gauge.dec();
        assert_eq!(gauge.get(), 10);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new();
        histogram.observe(0.05);
        histogram.observe(0.15);
        histogram.observe(0.5);

        assert_eq!(histogram.count(), 3);
    }

    #[test]
    fn test_metrics_prometheus_output() {
        let metrics = Metrics::new();
        metrics.connections_total.inc();
        metrics.active_connections.set(5);

        let output = metrics.to_prometheus();
        assert!(output.contains("rwire_connections_total"));
        assert!(output.contains("rwire_active_connections"));
        assert!(output.contains("# TYPE"));
        assert!(output.contains("# HELP"));
    }

    #[test]
    fn test_metrics_with_labels() {
        let metrics = Metrics::new();
        metrics.add_label("instance", "server1");
        metrics.add_label("env", "production");

        let output = metrics.to_prometheus();
        assert!(output.contains("instance=\"server1\""));
        assert!(output.contains("env=\"production\""));
    }
}
