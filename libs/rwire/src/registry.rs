//! Connection registry for tracking active connections.
//!
//! Provides connection counting, rate limiting by IP, and admission control.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

/// Registry for tracking active connections and enforcing limits.
///
/// Thread-safe registry that tracks:
/// - Total connection count
/// - Connections per IP address
///
/// Used for admission control and health checks.
pub struct ConnectionRegistry {
    /// Total active connection count.
    total: AtomicUsize,
    /// Connections per IP address.
    per_ip: RwLock<HashMap<IpAddr, usize>>,
}

impl ConnectionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            total: AtomicUsize::new(0),
            per_ip: RwLock::new(HashMap::new()),
        }
    }

    /// Get the total number of active connections.
    pub fn total_connections(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    /// Get the number of connections from a specific IP.
    pub fn connections_from_ip(&self, ip: IpAddr) -> usize {
        self.per_ip
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&ip)
            .copied()
            .unwrap_or(0)
    }

    /// Register a new connection from the given IP.
    ///
    /// Returns the guard that will automatically unregister the connection
    /// when dropped.
    pub fn register(&self, ip: IpAddr) -> ConnectionGuard<'_> {
        // Increment total
        self.total.fetch_add(1, Ordering::Relaxed);

        // Increment per-IP count
        *self
            .per_ip
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(ip)
            .or_insert(0) += 1;

        ConnectionGuard { registry: self, ip }
    }

    /// Unregister a connection (called by guard on drop).
    fn unregister(&self, ip: IpAddr) {
        // Decrement total
        self.total.fetch_sub(1, Ordering::Relaxed);

        // Decrement per-IP count
        let mut map = self.per_ip.write().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = map.get_mut(&ip) {
            if *count > 1 {
                *count -= 1;
            } else {
                map.remove(&ip);
            }
        }
    }

    /// Check if a new connection from the given IP can be admitted.
    ///
    /// Returns Ok(()) if the connection is allowed, Err with reason otherwise.
    pub fn check_admission(
        &self,
        ip: IpAddr,
        max_total: usize,
        max_per_ip: usize,
    ) -> Result<(), AdmissionError> {
        let total = self.total_connections();
        if total >= max_total {
            return Err(AdmissionError::AtCapacity);
        }

        let from_ip = self.connections_from_ip(ip);
        if from_ip >= max_per_ip {
            return Err(AdmissionError::TooManyFromIp);
        }

        Ok(())
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that unregisters a connection when dropped.
pub struct ConnectionGuard<'a> {
    registry: &'a ConnectionRegistry,
    ip: IpAddr,
}

impl<'a> Drop for ConnectionGuard<'a> {
    fn drop(&mut self) {
        self.registry.unregister(self.ip);
    }
}

/// Reasons a connection may be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// Server is at maximum connection capacity.
    AtCapacity,
    /// Too many connections from this IP address.
    TooManyFromIp,
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdmissionError::AtCapacity => write!(f, "server at capacity"),
            AdmissionError::TooManyFromIp => write!(f, "too many connections from IP"),
        }
    }
}

impl std::error::Error for AdmissionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_empty_registry() {
        let registry = ConnectionRegistry::new();
        assert_eq!(registry.total_connections(), 0);
    }

    #[test]
    fn test_register_unregister() {
        let registry = ConnectionRegistry::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        {
            let _guard = registry.register(ip);
            assert_eq!(registry.total_connections(), 1);
            assert_eq!(registry.connections_from_ip(ip), 1);
        }

        assert_eq!(registry.total_connections(), 0);
        assert_eq!(registry.connections_from_ip(ip), 0);
    }

    #[test]
    fn test_multiple_connections() {
        let registry = ConnectionRegistry::new();
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let _g1 = registry.register(ip1);
        let _g2 = registry.register(ip1);
        let _g3 = registry.register(ip2);

        assert_eq!(registry.total_connections(), 3);
        assert_eq!(registry.connections_from_ip(ip1), 2);
        assert_eq!(registry.connections_from_ip(ip2), 1);
    }

    #[test]
    fn test_admission_check() {
        let registry = ConnectionRegistry::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Should be admitted
        assert!(registry.check_admission(ip, 10, 5).is_ok());

        // Register some connections
        let _g1 = registry.register(ip);
        let _g2 = registry.register(ip);

        // Still OK
        assert!(registry.check_admission(ip, 10, 5).is_ok());

        // Fill up per-IP limit
        let _g3 = registry.register(ip);
        let _g4 = registry.register(ip);
        let _g5 = registry.register(ip);

        // Should be rejected - too many from IP
        assert_eq!(
            registry.check_admission(ip, 10, 5),
            Err(AdmissionError::TooManyFromIp)
        );

        // Different IP should still be allowed
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert!(registry.check_admission(ip2, 10, 5).is_ok());
    }

    #[test]
    fn test_admission_at_capacity() {
        let registry = ConnectionRegistry::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let _g1 = registry.register(ip);
        let _g2 = registry.register(ip);

        // At max total capacity (2)
        assert_eq!(
            registry.check_admission(ip, 2, 10),
            Err(AdmissionError::AtCapacity)
        );
    }
}
