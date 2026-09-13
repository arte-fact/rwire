//! Parse model metadata needed for VRAM estimation.
//!
//! For GGUF files we read the header's key–value metadata (layer/head counts,
//! embedding length) and the file size; for Hugging Face directories we read
//! `config.json` and sum the weight shard sizes. Both feed [`crate::vram`].
//!
//! The GGUF reader walks metadata entries with `Seek`, skipping large array
//! values (e.g. tokenizer vocabularies) without loading them, and stops as soon
//! as every field of interest has been found.

// Rust guideline compliant 2026-02-21

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Architecture parameters used to size weights and the KV cache.
#[derive(Debug, Clone, Copy)]
pub struct ModelMeta {
    /// Transformer block count.
    pub n_layers: u32,
    /// Attention head count (for `head_dim = embedding_dim / n_heads`).
    pub n_heads: u32,
    /// Key/value head count (≤ `n_heads` under grouped-query attention).
    pub n_kv_heads: u32,
    /// Embedding (hidden) dimension.
    pub embedding_dim: u32,
    /// Maximum trained context length.
    pub max_ctx: u32,
    /// On-disk weight size in mebibytes (≈ VRAM for the weights).
    pub weights_mb: u64,
}

impl ModelMeta {
    /// Derived per-head dimension; falls back to 128 when head count is unknown.
    pub fn head_dim(&self) -> u32 {
        self.embedding_dim
            .checked_div(self.n_heads)
            .filter(|d| *d > 0)
            .unwrap_or(128)
    }
}

/// GGUF metadata value type tags (see the GGUF spec).
mod gguf_type {
    pub const UINT8: u32 = 0;
    pub const INT8: u32 = 1;
    pub const UINT16: u32 = 2;
    pub const INT16: u32 = 3;
    pub const UINT32: u32 = 4;
    pub const INT32: u32 = 5;
    pub const FLOAT32: u32 = 6;
    pub const BOOL: u32 = 7;
    pub const STRING: u32 = 8;
    pub const ARRAY: u32 = 9;
    pub const UINT64: u32 = 10;
    pub const INT64: u32 = 11;
    pub const FLOAT64: u32 = 12;
}

/// Byte width of a fixed-size GGUF scalar type, or `None` for variable types.
const fn scalar_width(t: u32) -> Option<u64> {
    Some(match t {
        gguf_type::UINT8 | gguf_type::INT8 | gguf_type::BOOL => 1,
        gguf_type::UINT16 | gguf_type::INT16 => 2,
        gguf_type::UINT32 | gguf_type::INT32 | gguf_type::FLOAT32 => 4,
        gguf_type::UINT64 | gguf_type::INT64 | gguf_type::FLOAT64 => 8,
        _ => return None,
    })
}

/// Read metadata for a model path of the given format.
pub fn read(path: &str, is_gguf: bool) -> Option<ModelMeta> {
    if is_gguf {
        read_gguf(Path::new(path))
    } else {
        read_hf(Path::new(path))
    }
}

/// Parse a GGUF file's header metadata + size into [`ModelMeta`].
fn read_gguf(path: &Path) -> Option<ModelMeta> {
    // Total weights = this file, plus the other shards of a split GGUF.
    let weights_mb = gguf_total_bytes(path) / (1024 * 1024);
    let mut r = BufReader::new(File::open(path).ok()?);

    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).ok()?;
    if &magic != b"GGUF" {
        return None;
    }
    let _version = read_u32(&mut r)?;
    let _tensor_count = read_u64(&mut r)?;
    let kv_count = read_u64(&mut r)?;

    // Collect the scalar values of interest by key suffix; the architecture
    // prefix (e.g. "llama") is not known until "general.architecture" is read.
    let mut arch: Option<String> = None;
    let mut block_count = None;
    let mut head_count = None;
    let mut head_count_kv = None;
    let mut embedding_length = None;
    let mut context_length = None;

    for _ in 0..kv_count.min(4096) {
        let key = read_string(&mut r)?;
        let vtype = read_u32(&mut r)?;

        // Only a handful of scalar keys matter; everything else is skipped.
        let want_scalar = key == "general.architecture"
            || key.ends_with(".block_count")
            || key.ends_with(".attention.head_count")
            || key.ends_with(".attention.head_count_kv")
            || key.ends_with(".embedding_length")
            || key.ends_with(".context_length");

        if !want_scalar {
            skip_value(&mut r, vtype)?;
        } else if key == "general.architecture" {
            arch = Some(read_value_string(&mut r, vtype)?);
        } else {
            let v = read_value_u64(&mut r, vtype)?;
            let v32 = u32::try_from(v).unwrap_or(u32::MAX);
            if key.ends_with(".block_count") {
                block_count = Some(v32);
            } else if key.ends_with(".attention.head_count_kv") {
                head_count_kv = Some(v32);
            } else if key.ends_with(".attention.head_count") {
                head_count = Some(v32);
            } else if key.ends_with(".embedding_length") {
                embedding_length = Some(v32);
            } else if key.ends_with(".context_length") {
                context_length = Some(v32);
            }
        }

        // Stop early once everything needed has been seen (keeps us away from
        // the large tokenizer arrays that usually follow).
        if arch.is_some()
            && block_count.is_some()
            && head_count.is_some()
            && head_count_kv.is_some()
            && embedding_length.is_some()
            && context_length.is_some()
        {
            break;
        }
    }

    let n_layers = block_count?;
    let embedding_dim = embedding_length?;
    let n_heads = head_count.unwrap_or(0);
    // GQA models omit head_count_kv when it equals head_count.
    let n_kv_heads = head_count_kv.or(head_count).unwrap_or(0);
    Some(ModelMeta {
        n_layers,
        n_heads,
        n_kv_heads,
        embedding_dim,
        max_ctx: context_length.unwrap_or(0),
        weights_mb,
    })
}

/// Parse a Hugging Face `config.json` + weight sizes into [`ModelMeta`].
fn read_hf(dir: &Path) -> Option<ModelMeta> {
    let text = std::fs::read_to_string(dir.join("config.json")).ok()?;
    let cfg: serde_json::Value = serde_json::from_str(&text).ok()?;
    let u = |k: &str| {
        cfg.get(k)
            .and_then(serde_json::Value::as_u64)
            .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
    };

    let n_heads = u("num_attention_heads").unwrap_or(0);
    let weights_mb = weight_dir_size(dir) / (1024 * 1024);
    Some(ModelMeta {
        n_layers: u("num_hidden_layers")?,
        n_heads,
        n_kv_heads: u("num_key_value_heads")
            .or(Some(n_heads))
            .filter(|v| *v > 0)?,
        embedding_dim: u("hidden_size")?,
        max_ctx: u("max_position_embeddings").unwrap_or(0),
        weights_mb,
    })
}

/// Total on-disk bytes for a GGUF, summing all shards of a split model
/// (`…-00001-of-00003.gguf` + siblings); a single file returns its own size.
fn gguf_total_bytes(path: &Path) -> u64 {
    let own = path.metadata().map_or(0, |m| m.len());
    let (Some(name), Some(dir)) = (path.file_name().and_then(|n| n.to_str()), path.parent()) else {
        return own;
    };
    // Split shard names look like "<prefix>-NNNNN-of-MMMMM.gguf".
    let Some(of_pos) = name.rfind("-of-") else {
        return own;
    };
    let suffix = &name[of_pos..]; // "-of-MMMMM.gguf"
    let before = &name[..of_pos];
    let Some(dash) = before.rfind('-') else {
        return own;
    };
    let prefix = &before[..dash]; // shared base name
    let Ok(entries) = std::fs::read_dir(dir) else {
        return own;
    };
    let total: u64 = entries
        .flatten()
        .filter_map(|e| {
            let n = e.file_name();
            let n = n.to_str()?;
            (n.starts_with(prefix) && n.ends_with(suffix))
                .then(|| e.metadata().ok().map(|m| m.len()))
                .flatten()
        })
        .sum();
    if total > 0 { total } else { own }
}

/// Sum the sizes of weight shards (`*.safetensors` / `*.bin`) under `dir`.
fn weight_dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let ext = p.extension()?.to_str()?;
            if ext == "safetensors" || ext == "bin" {
                p.metadata().ok().map(|m| m.len())
            } else {
                None
            }
        })
        .sum()
}

fn read_u32(r: &mut impl Read) -> Option<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).ok()?;
    Some(u32::from_le_bytes(b))
}

fn read_u64(r: &mut impl Read) -> Option<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).ok()?;
    Some(u64::from_le_bytes(b))
}

/// Read a GGUF string (u64 length + UTF-8 bytes).
fn read_string(r: &mut impl Read) -> Option<String> {
    let len = usize::try_from(read_u64(r)?).unwrap_or(usize::MAX);
    if len > 1 << 20 {
        return None; // guard against a corrupt length
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// Read a scalar value of `vtype` as a `u64` (for the integer fields we need).
fn read_value_u64<R: Read>(r: &mut R, vtype: u32) -> Option<u64> {
    match vtype {
        gguf_type::UINT8 | gguf_type::INT8 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b).ok()?;
            Some(u64::from(b[0]))
        }
        gguf_type::UINT16 | gguf_type::INT16 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b).ok()?;
            Some(u64::from(u16::from_le_bytes(b)))
        }
        gguf_type::UINT32 | gguf_type::INT32 => Some(u64::from(read_u32(r)?)),
        gguf_type::UINT64 | gguf_type::INT64 => read_u64(r),
        _ => None,
    }
}

/// Read a GGUF string value (only valid when `vtype` is STRING).
fn read_value_string<R: Read>(r: &mut R, vtype: u32) -> Option<String> {
    if vtype == gguf_type::STRING {
        read_string(r)
    } else {
        None
    }
}

/// Skip a GGUF value of any type using `Seek` for fixed-size payloads.
fn skip_value<R: Read + Seek>(r: &mut R, vtype: u32) -> Option<()> {
    match vtype {
        gguf_type::STRING => {
            let len = read_u64(r)?;
            r.seek(SeekFrom::Current(i64::try_from(len).unwrap_or(i64::MAX)))
                .ok()?;
        }
        gguf_type::ARRAY => {
            let elem_type = read_u32(r)?;
            let count = read_u64(r)?;
            if let Some(w) = scalar_width(elem_type) {
                r.seek(SeekFrom::Current(
                    i64::try_from(w * count).unwrap_or(i64::MAX),
                ))
                .ok()?;
            } else if elem_type == gguf_type::STRING {
                for _ in 0..count {
                    let len = read_u64(r)?;
                    r.seek(SeekFrom::Current(i64::try_from(len).unwrap_or(i64::MAX)))
                        .ok()?;
                }
            } else {
                return None; // nested arrays: unsupported, bail
            }
        }
        other => {
            let w = scalar_width(other)?;
            r.seek(SeekFrom::Current(i64::try_from(w).unwrap_or(i64::MAX)))
                .ok()?;
        }
    }
    Some(())
}
