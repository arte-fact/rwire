//! Per-engine command construction: resolve the executable for an
//! engine+backend, build the argv, and spawn the child process.

// Rust guideline compliant 2026-02-21

use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::snapshot::{EngineKind, LlmNode, MaterialClass};

/// Compute backend a material class maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cuda,
    Hip,
    Cpu,
}

impl Backend {
    const fn of(material: MaterialClass) -> Self {
        match material {
            MaterialClass::Nvidia => Self::Cuda,
            MaterialClass::Amd => Self::Hip,
            MaterialClass::Cpu => Self::Cpu,
        }
    }
}

/// A resolved engine executable + optional library path (its "base env").
pub struct EngineExec {
    exec: String,
    lib_path: Option<String>,
}

/// Everything the supervisor needs to launch one node.
pub struct LaunchSpec {
    pub id: u64,
    pub engine: EngineKind,
    pub material: MaterialClass,
    pub devices: Vec<u32>,
    pub model: String,
    pub ctx: Option<u32>,
    pub port: u16,
    /// Dynamic flag rows as `(flag, value)`; value is empty for bare toggles.
    pub flags: Vec<(String, String)>,
}

impl LaunchSpec {
    /// Build a launch spec from a node (returns `None` if it isn't launchable).
    pub fn from_node(node: &LlmNode) -> Option<Self> {
        let engine = node.engine?;
        if node.model.is_empty() {
            return None;
        }
        let flags = node
            .flags
            .iter()
            .filter(|f| !f.flag.trim().is_empty())
            .map(|f| (f.flag.trim().to_string(), f.value.trim().to_string()))
            .collect();
        Some(Self {
            id: node.id,
            engine,
            material: node.material,
            devices: node.device_ordinals(),
            model: node.model.clone(),
            ctx: node.ctx.trim().parse().ok(),
            port: node.port,
            flags,
        })
    }
}

/// Resolve, build, and spawn the engine process; returns an error string on failure.
pub fn spawn(spec: &LaunchSpec) -> Result<Child, String> {
    let backend = Backend::of(spec.material);
    let exec = resolve(spec.engine, backend).ok_or_else(|| {
        format!(
            "no executable configured for {} on {:?} (set the registry env var)",
            spec.engine.label(),
            backend
        )
    })?;

    let mut cmd = build_command(spec, &exec, backend);

    // Per-node log file so a full pipe buffer can't block the child.
    let log = crate::launcher::node_log_path(spec.id);
    if let Ok(file) = File::create(&log)
        && let Ok(err) = file.try_clone()
    {
        cmd.stdout(Stdio::from(file)).stderr(Stdio::from(err));
    }

    // Orphan backstop: ask the kernel to SIGTERM this child if the monitor dies
    // (covers SIGKILL/crash of the monitor and stale children on restart, which a
    // userspace signal handler can't). The supervisor thread lives for the whole
    // process, so parent-thread death ≈ process death.
    #[cfg(target_os = "linux")]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong);
            Ok(())
        });
    }

    cmd.spawn()
        .map_err(|e| format!("failed to start {}: {e}", exec.exec))
}

/// `ROCm` install root for AMD/HIP engines, from `ROCM_OVERRIDE` (default
/// `/opt/rocm`).
///
/// Note: on gfx906 (MI50) the stock `/opt/rocm` (7.2.x) dropped the rocBLAS
/// Tensile kernels and SIGABRTs; point `ROCM_OVERRIDE` at a build that still
/// ships them (e.g. `/opt/rocm-host` 7.1.1). Configure this in `.env`.
pub fn rocm_root() -> String {
    std::env::var("ROCM_OVERRIDE").unwrap_or_else(|_| "/opt/rocm".to_string())
}

/// Map (engine, backend) to an executable, fully from env vars (no paths baked
/// into the binary). Each pair reads its `*_BIN` (executable) and optional
/// `*_LIBS` (`LD_LIBRARY_PATH` prefix); an unset `*_BIN` means "not configured".
/// See `.env.example` for the full list.
pub fn resolve(engine: EngineKind, backend: Backend) -> Option<EngineExec> {
    let env_exec = |bin: &str, libs: &str| {
        std::env::var(bin).ok().map(|exec| EngineExec {
            exec,
            lib_path: std::env::var(libs).ok(),
        })
    };
    match (engine, backend) {
        (EngineKind::LlamaCpp, Backend::Hip) => env_exec("LLAMACPP_HIP_BIN", "LLAMACPP_HIP_LIBS"),
        (EngineKind::LlamaCpp, Backend::Cuda) => {
            env_exec("LLAMACPP_CUDA_BIN", "LLAMACPP_CUDA_LIBS")
        }
        (EngineKind::LlamaCpp, Backend::Cpu) => env_exec("LLAMACPP_CPU_BIN", "LLAMACPP_CPU_LIBS"),
        (EngineKind::Flambeau, Backend::Hip) => env_exec("FLAMBEAU_HIP_BIN", "FLAMBEAU_HIP_LIBS"),
        (EngineKind::Flambeau, Backend::Cuda) => {
            env_exec("FLAMBEAU_CUDA_BIN", "FLAMBEAU_CUDA_LIBS")
        }
        (EngineKind::Vllm, Backend::Cuda) => env_exec("VLLM_CUDA_BIN", "VLLM_CUDA_LIBS"),
        (EngineKind::Vllm, Backend::Cpu) => env_exec("VLLM_CPU_BIN", "VLLM_CPU_LIBS"),
        _ => None,
    }
}

/// Build the engine command (argv + env) for a launch spec.
pub fn build_command(spec: &LaunchSpec, exec: &EngineExec, backend: Backend) -> Command {
    let mut cmd = Command::new(&exec.exec);

    // Base library path, prepended to any inherited LD_LIBRARY_PATH.
    if let Some(libs) = &exec.lib_path {
        let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let value = if existing.is_empty() {
            libs.clone()
        } else {
            format!("{libs}:{existing}")
        };
        cmd.env("LD_LIBRARY_PATH", value);
    }

    // AMD/HIP: point rocBLAS at the overridden ROCm's Tensile kernels so gfx906
    // (MI50) GEMMs work (the system /opt/rocm dropped them → rocBLAS SIGABRT).
    if backend == Backend::Hip {
        let root = rocm_root();
        cmd.env(
            "ROCBLAS_TENSILE_LIBPATH",
            format!("{root}/lib/rocblas/library"),
        );
        cmd.env("ROCM_PATH", &root);
        cmd.env("HIP_PATH", &root);
    }

    let mask = spec
        .devices
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let port = spec.port.to_string();

    match spec.engine {
        EngineKind::LlamaCpp => {
            cmd.args([
                "-m",
                &spec.model,
                "--host",
                "0.0.0.0",
                "--port",
                &port,
                "--metrics",
            ]);
            if backend == Backend::Cpu {
                cmd.args(["-ngl", "0"]);
            } else {
                cmd.args(["-ngl", "99"]);
                set_device_mask(&mut cmd, backend, &mask);
                if spec.devices.len() > 1 {
                    cmd.args(["--split-mode", "layer"]);
                }
            }
            if let Some(c) = spec.ctx {
                cmd.args(["-c", &c.to_string()]);
            }
        }
        EngineKind::Flambeau => {
            // flambeau is single-vendor (HIP here) and takes explicit --devices.
            cmd.arg("serve")
                .args(["--model", &spec.model, "--port", &port]);
            cmd.args(["--devices", &format!("hip:{mask}")]);
            if let Some(c) = spec.ctx {
                cmd.args(["--ctx-cap", &c.to_string()]);
            }
        }
        EngineKind::Vllm => {
            cmd.args(["serve", &spec.model, "--host", "0.0.0.0", "--port", &port]);
            cmd.args([
                "--tensor-parallel-size",
                &spec.devices.len().max(1).to_string(),
            ]);
            set_device_mask(&mut cmd, backend, &mask);
            if let Some(c) = spec.ctx {
                cmd.args(["--max-model-len", &c.to_string()]);
            }
        }
    }

    // User-specified dynamic flags come last, so an explicit value overrides the
    // engine-specific defaults set above. A blank value means a bare toggle.
    for (flag, value) in &spec.flags {
        if value.is_empty() {
            cmd.arg(flag);
        } else {
            cmd.args([flag, value]);
        }
    }
    cmd
}

/// Set the vendor device-mask env var on the child (process-wide, pre-enumeration).
pub fn set_device_mask(cmd: &mut Command, backend: Backend, mask: &str) {
    match backend {
        Backend::Cuda => {
            cmd.env("CUDA_VISIBLE_DEVICES", mask);
        }
        Backend::Hip => {
            cmd.env("HIP_VISIBLE_DEVICES", mask)
                .env("ROCR_VISIBLE_DEVICES", mask);
        }
        Backend::Cpu => {}
    }
}

/// One `GET /health` probe; true only on an HTTP 200.
///
/// llama.cpp returns 503 while loading and 200 when ready; flambeau returns 200
/// once serving. The reconcile loop polls this until ready or the process exits,
/// so no separate timeout is needed (a crash is caught by `try_wait`).
pub fn health_ok(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let req = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).contains(" 200")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_of(cmd: &Command) -> Vec<String> {
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn has_pair(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2).any(|w| w[0] == flag && w[1] == value)
    }

    #[test]
    fn build_command_appends_dynamic_flags() {
        let spec = LaunchSpec {
            id: 0,
            engine: EngineKind::LlamaCpp,
            material: MaterialClass::Amd,
            devices: vec![0],
            model: "m.gguf".to_string(),
            ctx: Some(4096),
            port: 8090,
            flags: vec![
                ("--flash-attn".to_string(), "on".to_string()),
                ("--temp".to_string(), "1.0".to_string()),
                // A bare toggle (empty value) → flag only, no value token.
                ("--jinja".to_string(), String::new()),
            ],
        };
        let exec = EngineExec {
            exec: "llama-server".to_string(),
            lib_path: None,
        };
        let args = args_of(&build_command(&spec, &exec, Backend::Hip));

        // Base llama.cpp args still present.
        assert!(has_pair(&args, "-m", "m.gguf"));
        assert!(has_pair(&args, "-c", "4096"));
        // Value-bearing dynamic flags appended with their values.
        assert!(has_pair(&args, "--flash-attn", "on"));
        assert!(has_pair(&args, "--temp", "1.0"));
        // The bare toggle is present with no following value.
        let jinja = args.iter().position(|a| a == "--jinja").unwrap();
        assert!(args.get(jinja + 1).is_none_or(|next| next.starts_with('-')));
        // Dynamic flags come after the managed ones.
        let temp = args.iter().position(|a| a == "--temp").unwrap();
        let m = args.iter().position(|a| a == "-m").unwrap();
        assert!(temp > m);
    }
}
