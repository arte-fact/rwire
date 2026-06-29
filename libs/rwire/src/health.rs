//! Health check endpoints for load balancers and orchestration.

use async_std::io::WriteExt;
use async_std::net::TcpStream;
use std::io;

use crate::registry::ConnectionRegistry;

/// Health status of the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Server is healthy and operating normally.
    Healthy,
    /// Server is degraded but still accepting connections.
    Degraded,
    /// Server is unhealthy and should not receive traffic.
    Unhealthy,
}

impl HealthStatus {
    /// Get the HTTP status code for this health status.
    pub fn status_code(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 200,
            HealthStatus::Unhealthy => 503,
        }
    }

    /// Get the status text for this health status.
    pub fn status_text(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        }
    }
}

/// Health check response data.
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub active_connections: usize,
    pub max_connections: usize,
}

impl HealthResponse {
    /// Create a new health response.
    pub fn new(registry: &ConnectionRegistry, max_connections: usize) -> Self {
        let active = registry.total_connections();
        let utilization = if max_connections > 0 {
            (active as f64 / max_connections as f64) * 100.0
        } else {
            0.0
        };

        let status = if utilization >= 95.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            status,
            active_connections: active,
            max_connections,
        }
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"status":"{}","active_connections":{},"max_connections":{}}}"#,
            self.status.status_text(),
            self.active_connections,
            self.max_connections
        )
    }
}

/// Readiness check response.
#[derive(Debug, Clone)]
pub struct ReadyResponse {
    pub ready: bool,
    pub reason: Option<String>,
}

impl ReadyResponse {
    /// Create a ready response when the server can accept connections.
    pub fn ready() -> Self {
        Self {
            ready: true,
            reason: None,
        }
    }

    /// Create a not-ready response with a reason.
    pub fn not_ready(reason: impl Into<String>) -> Self {
        Self {
            ready: false,
            reason: Some(reason.into()),
        }
    }

    /// Check readiness based on connection registry state.
    pub fn from_registry(registry: &ConnectionRegistry, max_connections: usize) -> Self {
        let active = registry.total_connections();
        if active >= max_connections {
            Self::not_ready("at_capacity")
        } else {
            Self::ready()
        }
    }

    /// Get the HTTP status code.
    pub fn status_code(&self) -> u16 {
        if self.ready {
            200
        } else {
            503
        }
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        match &self.reason {
            Some(reason) => format!(r#"{{"ready":false,"reason":"{}"}}"#, reason),
            None => r#"{"ready":true}"#.to_string(),
        }
    }
}

/// Serve a health check response.
pub async fn serve_health(
    mut stream: TcpStream,
    registry: &ConnectionRegistry,
    max_connections: usize,
) -> io::Result<()> {
    let response = HealthResponse::new(registry, max_connections);
    let body = response.to_json();

    let http_response = format!(
        "HTTP/1.1 {} OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        response.status.status_code(),
        body.len(),
        body
    );

    stream.write_all(http_response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Serve a readiness check response.
pub async fn serve_ready(
    mut stream: TcpStream,
    registry: &ConnectionRegistry,
    max_connections: usize,
) -> io::Result<()> {
    let response = ReadyResponse::from_registry(registry, max_connections);
    let body = response.to_json();

    let http_response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        response.status_code(),
        if response.ready {
            "OK"
        } else {
            "Service Unavailable"
        },
        body.len(),
        body
    );

    stream.write_all(http_response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Serve a static byte body with a given content type and a long cache header.
/// Used for PWA assets (manifest, service worker, icons).
pub async fn serve_static(
    mut stream: TcpStream,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: public, max-age=3600\r\n\
         Connection: close\r\n\
         \r\n",
        content_type,
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Serve a Prometheus text-format metrics response (`GET /metrics`).
pub async fn serve_metrics(mut stream: TcpStream, body: &str) -> io::Result<()> {
    let http_response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        body.len(),
        body
    );

    stream.write_all(http_response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Serve a 503 Service Unavailable response for admission control.
pub async fn serve_unavailable(mut stream: TcpStream, reason: &str) -> io::Result<()> {
    let body = format!(r#"{{"error":"service_unavailable","reason":"{}"}}"#, reason);

    let http_response = format!(
        "HTTP/1.1 503 Service Unavailable\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        body.len(),
        body
    );

    stream.write_all(http_response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_codes() {
        assert_eq!(HealthStatus::Healthy.status_code(), 200);
        assert_eq!(HealthStatus::Degraded.status_code(), 200);
        assert_eq!(HealthStatus::Unhealthy.status_code(), 503);
    }

    #[test]
    fn test_ready_response_json() {
        let ready = ReadyResponse::ready();
        assert_eq!(ready.to_json(), r#"{"ready":true}"#);

        let not_ready = ReadyResponse::not_ready("at_capacity");
        assert_eq!(
            not_ready.to_json(),
            r#"{"ready":false,"reason":"at_capacity"}"#
        );
    }

    #[test]
    fn test_health_response_json() {
        let registry = ConnectionRegistry::new();
        let response = HealthResponse::new(&registry, 10000);
        assert!(response.to_json().contains("healthy"));
        assert!(response.to_json().contains("active_connections"));
    }
}
