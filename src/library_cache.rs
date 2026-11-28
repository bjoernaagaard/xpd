//! Content-addressed incremental vector cache for local and specialized library indexes.
//!
//! Stores quantized `DIMENSIONS`-byte `EmbeddingGemma` vectors under
//! `$XDG_CACHE_HOME/xpd/indexes/<sha256>.vectors.bin` using the exact same wire format and
//! FNV-1a text hashing as `indexes/vectors.bin`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::embed::{Model, cache_folder};
use crate::index::Entry;
use crate::search::{DIMENSIONS, VECTORS_HEADER, text_hash};

/// Builds the summary text embedded for a local or specialized entry, enriching it with the
/// defining module path when the symbol is re-exported under a flatter public alias.
pub(crate) fn entry_embed_text(entry: &Entry) -> String {
    match entry.defined_in.as_deref() {
        Some(module)
            if !module.is_empty() && module != entry.package && !entry.name.starts_with(module) =>
        {
            if entry.summary.is_empty() {
                format!("module {module}")
            } else {
                format!("{module}. {}", entry.summary)
            }
        }
        _ => entry.summary.clone(),
    }
}

/// Returns the cache file path for `cache_key` under `$XDG_CACHE_HOME/xpd/indexes/`.
pub(crate) fn cache_path(cache_key: &str) -> Result<PathBuf> {
    let digest = Sha256::digest(cache_key.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(cache_folder()?
        .join("indexes")
        .join(format!("{hex}.vectors.bin")))
}

/// Loads any valid, text-hash-matching cached vectors for `entries` without touching the model.
/// Entries with missing or stale vectors receive all-zero arrays.
pub(crate) fn load_cached_vectors(cache_key: &str, entries: &[Entry]) -> Vec<[u8; DIMENSIONS]> {
    let Ok(path) = cache_path(cache_key) else {
        return vec![[0; DIMENSIONS]; entries.len()];
    };
    let Ok(bytes) = fs::read(&path) else {
        return vec![[0; DIMENSIONS]; entries.len()];
    };
    let Some(stored) = parse_vectors(&bytes) else {
        return vec![[0; DIMENSIONS]; entries.len()];
    };
    entries
        .iter()
        .map(|entry| {
            let text = entry_embed_text(entry);
            let expected = text_hash(&entry.name, &text);
            stored
                .get(entry.name.as_str())
                .filter(|(hash, _)| *hash == expected)
                .map_or([0; DIMENSIONS], |(_, vector)| *vector)
        })
        .collect()
}

/// Embeds only entries whose vectors are currently all-zero, updates `vectors` in place, and
/// persists the refreshed `.vectors.bin` file atomically. Returns the number of newly embedded
/// entries.
pub(crate) fn ensure_cached_vectors(
    cache_key: &str,
    entries: &[Entry],
    vectors: &mut [[u8; DIMENSIONS]],
    model: &Model,
) -> Result<usize> {
    let missing_indices: Vec<usize> = vectors
        .iter()
        .enumerate()
        .filter(|(_, vector)| vector.iter().all(|&byte| byte == 0))
        .map(|(index, _)| index)
        .collect();
    if missing_indices.is_empty() {
        return Ok(0);
    }
    let embed_texts: Vec<String> = missing_indices
        .iter()
        .map(|&index| entry_embed_text(&entries[index]))
        .collect();
    let pairs: Vec<(&str, &str)> = missing_indices
        .iter()
        .zip(&embed_texts)
        .map(|(&index, text)| (entries[index].name.as_str(), text.as_str()))
        .collect();
    let embedded = model.documents(&pairs)?;
    for (&index, raw_vector) in missing_indices.iter().zip(embedded) {
        vectors[index] = quantize(&raw_vector);
    }
    write_vectors(cache_key, entries, vectors)?;
    Ok(missing_indices.len())
}

/// Counts how many entries in `vectors` currently hold a non-zero embedding vector.
pub(crate) fn cached_vector_count(vectors: &[[u8; DIMENSIONS]]) -> usize {
    vectors
        .iter()
        .filter(|vector| vector.iter().any(|&byte| byte != 0))
        .count()
}

fn quantize(vector: &[f32]) -> [u8; DIMENSIONS] {
    let largest = vector.iter().fold(f32::EPSILON, |max, v| max.max(v.abs()));
    let mut out = [0_u8; DIMENSIONS];
    for (slot, &value) in out.iter_mut().zip(vector) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::as_conversions,
            reason = "rounded value within -127..=127"
        )]
        let byte = (value / largest * 127.0).round() as i8;
        *slot = byte.cast_unsigned();
    }
    out
}

fn parse_vectors(bytes: &[u8]) -> Option<BTreeMap<&str, (u64, [u8; DIMENSIONS])>> {
    let mut rest = bytes.strip_prefix(VECTORS_HEADER)?;
    let mut found = BTreeMap::new();
    while let Some((length, tail)) = rest.split_first_chunk::<2>() {
        let (name_bytes, tail) = tail.split_at_checked(usize::from(u16::from_le_bytes(*length)))?;
        let (hash_bytes, tail) = tail.split_first_chunk::<8>()?;
        let (vector_bytes, tail) = tail.split_at_checked(DIMENSIONS)?;
        let name = std::str::from_utf8(name_bytes).ok()?;
        let vector: [u8; DIMENSIONS] = vector_bytes.try_into().ok()?;
        found.insert(name, (u64::from_le_bytes(*hash_bytes), vector));
        rest = tail;
    }
    rest.is_empty().then_some(found)
}

fn write_vectors(cache_key: &str, entries: &[Entry], vectors: &[[u8; DIMENSIONS]]) -> Result<()> {
    let path = cache_path(cache_key)?;
    let folder = path
        .parent()
        .context("vector cache path has a parent directory")?;
    fs::create_dir_all(folder)
        .with_context(|| format!("cannot create vector cache folder {}", folder.display()))?;
    let mut data = VECTORS_HEADER.to_vec();
    for (entry, vector) in entries.iter().zip(vectors) {
        if vector.iter().all(|&byte| byte == 0) {
            continue;
        }
        let text = entry_embed_text(entry);
        let hash = text_hash(&entry.name, &text);
        data.extend(u16::try_from(entry.name.len())?.to_le_bytes());
        data.extend(entry.name.as_bytes());
        data.extend(hash.to_le_bytes());
        data.extend_from_slice(vector);
    }
    let temp = folder.join(format!(
        "{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("index"),
        std::process::id()
    ));
    fs::write(&temp, data)
        .with_context(|| format!("cannot write temporary vector cache {}", temp.display()))?;
    fs::rename(&temp, &path)
        .with_context(|| format!("cannot replace vector cache {}", path.display()))
}
