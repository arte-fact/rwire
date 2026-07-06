//! CPU model and utilization sampled from `/proc`.
//!
//! Utilization needs two `/proc/stat` samples; [`Cpu`] retains the previous
//! reading and reports the busy fraction since the last [`Cpu::sample`].

/// Cumulative CPU jiffies for one core or the aggregate `cpu` line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuTimes {
    /// Jiffies spent idle (idle + iowait).
    pub idle: u64,
    /// Total jiffies across all states.
    pub total: u64,
}

impl CpuTimes {
    /// Busy fraction since an earlier sample, as a percentage (0-100).
    pub fn usage_since(self, prev: Self) -> f32 {
        let total_delta = self.total.saturating_sub(prev.total);
        if total_delta == 0 {
            return 0.0;
        }
        let idle_delta = self.idle.saturating_sub(prev.idle);
        let busy = total_delta.saturating_sub(idle_delta);
        (crate::convert::u64_f32(busy) / crate::convert::u64_f32(total_delta)) * 100.0
    }
}

/// Parse `/proc/stat` CPU lines into cumulative times.
///
/// Index 0 is the aggregate `cpu` line; following entries are `cpu0`, `cpu1`, …
/// in file order. Non-CPU lines are ignored.
pub fn parse_proc_stat(stat: &str) -> Vec<CpuTimes> {
    let mut out = Vec::new();
    for line in stat.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("cpu") else {
            continue;
        };
        // The label is "cpu" (aggregate) or "cpuN"; the suffix before the first
        // space must be empty or all digits, else it's an unrelated key.
        let Some((label_suffix, fields)) = rest.split_once(char::is_whitespace) else {
            continue;
        };
        if !label_suffix.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // Fields: user nice system idle iowait irq softirq steal guest guest_nice
        let nums: Vec<u64> = fields
            .split_whitespace()
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();
        if nums.len() < 4 {
            continue;
        }
        let idle = nums[3] + nums.get(4).copied().unwrap_or(0); // idle + iowait
        let total: u64 = nums.iter().sum();
        out.push(CpuTimes { idle, total });
    }
    out
}

/// Extract the CPU model name from `/proc/cpuinfo`, if present.
pub fn parse_cpu_model(cpuinfo: &str) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().to_string())
    })
}

/// CPU reader holding the model string and the previous utilization sample.
#[derive(Debug, Clone)]
pub struct Cpu {
    model: String,
    prev: Vec<CpuTimes>,
}

impl Cpu {
    /// Read the model name and take a baseline utilization sample.
    pub fn new() -> Self {
        let model = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| parse_cpu_model(&s))
            .unwrap_or_else(|| "Unknown CPU".to_string());
        Self {
            model,
            prev: read_stat(),
        }
    }

    /// The CPU model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Sample utilization since the previous call.
    ///
    /// Returns `(aggregate_percent, per_core_percent)`. The first call after
    /// [`Cpu::new`] reports utilization relative to construction time.
    pub fn sample(&mut self) -> (f32, Vec<f32>) {
        let cur = read_stat();
        let result = if !cur.is_empty() && cur.len() == self.prev.len() {
            let aggregate = cur[0].usage_since(self.prev[0]);
            let per_core = cur[1..]
                .iter()
                .zip(&self.prev[1..])
                .map(|(c, p)| c.usage_since(*p))
                .collect();
            (aggregate, per_core)
        } else {
            // Core count changed or no data: report zeros for this sample.
            (0.0, vec![0.0; cur.len().saturating_sub(1)])
        };
        self.prev = cur;
        result
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

fn read_stat() -> Vec<CpuTimes> {
    std::fs::read_to_string("/proc/stat")
        .map(|s| parse_proc_stat(&s))
        .unwrap_or_default()
}

/// hwmon chip names that report CPU package temperature.
const CPU_HWMON_CHIPS: [&str; 4] = ["k10temp", "coretemp", "zenpower", "cpu_thermal"];

/// Pick the most representative CPU temperature from labeled sensor readings.
///
/// Prefers whole-package sensors (`Tdie`/`Tctl`/`Package id 0`) over per-CCD
/// or per-core sensors, taking the hottest match so multi-die/multi-socket
/// systems report a meaningful peak. Falls back to the hottest unlabeled value.
pub fn pick_cpu_temp(readings: &[(String, f32)]) -> Option<f32> {
    const PRIORITY: [&str; 4] = ["Tdie", "Tctl", "Package id 0", "Tccd1"];
    for key in PRIORITY {
        if let Some(t) = readings
            .iter()
            .filter(|(label, _)| label == key)
            .map(|(_, t)| *t)
            .reduce(f32::max)
        {
            return Some(t);
        }
    }
    readings.iter().map(|(_, t)| *t).reduce(f32::max)
}

/// Read the CPU package temperature in degrees Celsius from hwmon, if exposed.
pub fn read_cpu_temp() -> Option<f32> {
    let mut readings = Vec::new();
    for entry in std::fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let base = entry.path();
        let name = std::fs::read_to_string(base.join("name")).unwrap_or_default();
        if !CPU_HWMON_CHIPS.contains(&name.trim()) {
            continue;
        }
        // hwmon sensor indices are 1-based and sparse; scan a small range.
        for i in 1..=16 {
            let Ok(raw) = std::fs::read_to_string(base.join(format!("temp{i}_input"))) else {
                continue;
            };
            let Ok(millidegrees) = raw.trim().parse::<f32>() else {
                continue;
            };
            let label = std::fs::read_to_string(base.join(format!("temp{i}_label")))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            readings.push((label, millidegrees / 1000.0));
        }
    }
    pick_cpu_temp(&readings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
cpu  100 0 50 800 50 0 0 0 0 0
cpu0 50 0 25 400 25 0 0 0 0 0
cpu1 50 0 25 400 25 0 0 0 0 0
intr 12345
ctxt 67890
";

    #[test]
    fn parses_aggregate_and_cores() {
        let times = parse_proc_stat(SAMPLE);
        assert_eq!(times.len(), 3); // aggregate + 2 cores
        // aggregate: idle = 800 + 50 = 850, total = 1000
        assert_eq!(
            times[0],
            CpuTimes {
                idle: 850,
                total: 1000
            }
        );
        assert_eq!(
            times[1],
            CpuTimes {
                idle: 425,
                total: 500
            }
        );
    }

    #[test]
    fn ignores_non_cpu_lines() {
        let times = parse_proc_stat("intr 1\nctxt 2\ncpufreq 3\n");
        assert!(times.is_empty());
    }

    #[test]
    fn usage_since_computes_busy_fraction() {
        let prev = CpuTimes {
            idle: 800,
            total: 1000,
        };
        let cur = CpuTimes {
            idle: 850,
            total: 1100,
        };
        // total_delta = 100, idle_delta = 50, busy = 50 -> 50%
        assert!((cur.usage_since(prev) - 50.0).abs() < 0.01);
    }

    #[test]
    fn usage_since_zero_when_no_progress() {
        let t = CpuTimes {
            idle: 10,
            total: 20,
        };
        assert!(t.usage_since(t).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_model_name() {
        let cpuinfo = "processor\t: 0\nmodel name\t: AMD Ryzen 9 5950X\nstepping\t: 0\n";
        assert_eq!(
            parse_cpu_model(cpuinfo).as_deref(),
            Some("AMD Ryzen 9 5950X")
        );
    }

    #[test]
    fn missing_model_name_is_none() {
        assert_eq!(parse_cpu_model("processor\t: 0\n"), None);
    }

    #[test]
    fn temp_prefers_package_label_and_takes_max() {
        let readings = vec![
            ("Tctl".to_string(), 70.0),
            ("Tccd1".to_string(), 73.0),
            ("Tctl".to_string(), 72.0),
        ];
        // Tctl has priority over Tccd1; hottest Tctl wins.
        assert_eq!(pick_cpu_temp(&readings), Some(72.0));
    }

    #[test]
    fn temp_falls_back_to_hottest_unlabeled() {
        let readings = vec![(String::new(), 40.0), (String::new(), 55.0)];
        assert_eq!(pick_cpu_temp(&readings), Some(55.0));
    }

    #[test]
    fn temp_none_when_empty() {
        assert_eq!(pick_cpu_temp(&[]), None);
    }
}
