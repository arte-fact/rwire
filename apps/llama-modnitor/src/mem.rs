//! System memory usage parsed from `/proc/meminfo`.

/// Total and used system memory in kibibytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Memory {
    /// Total physical memory in kibibytes.
    pub total_kib: u64,
    /// Memory in use (`MemTotal - MemAvailable`) in kibibytes.
    pub used_kib: u64,
}

/// Parse `/proc/meminfo` into total and used memory.
///
/// Used memory is `MemTotal - MemAvailable`, matching what tools like `free`
/// report as "used" (i.e. excluding reclaimable cache).
pub fn parse_meminfo(meminfo: &str) -> Memory {
    let mut total = 0u64;
    let mut available = 0u64;
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = first_number(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = first_number(rest);
        }
    }
    Memory {
        total_kib: total,
        used_kib: total.saturating_sub(available),
    }
}

fn first_number(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Read current memory usage from `/proc/meminfo`.
pub fn read() -> Memory {
    std::fs::read_to_string("/proc/meminfo")
        .map(|s| parse_meminfo(&s))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_total_and_used() {
        let meminfo = "\
MemTotal:       32000000 kB
MemFree:         1000000 kB
MemAvailable:   20000000 kB
Buffers:          500000 kB
";
        let mem = parse_meminfo(meminfo);
        assert_eq!(mem.total_kib, 32_000_000);
        assert_eq!(mem.used_kib, 12_000_000); // 32M - 20M
    }

    #[test]
    fn missing_fields_default_to_zero() {
        let mem = parse_meminfo("SomethingElse: 1 kB\n");
        assert_eq!(mem.total_kib, 0);
        assert_eq!(mem.used_kib, 0);
    }
}
