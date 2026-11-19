//! The embedding model that `find` ranks by meaning with: where its file lives, the one download
//! of it, and the vector of a text.
//!
//! The model is `EmbeddingGemma` 300M, from <https://ai.google.dev/gemma/docs/embeddinggemma/model_card>.
//! Its vectors are cut to their first `DIMENSIONS` values and normalised again, which the model
//! is trained for.

use std::env;
use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, ensure};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;

use crate::venv;

/// The values that `indexes/vectors.bin` keeps of each vector.
pub(crate) const DIMENSIONS: usize = 256;
const FILE: &str = "embeddinggemma-300M-Q8_0.gguf";
/// Pinned to a revision, because `indexes/vectors.bin` holds this file's vectors.
const URL: &str = "https://huggingface.co/ggml-org/embeddinggemma-300M-GGUF/resolve/0f741b5a6585bd53aeb15cd1372c56f2a0f65e12/embeddinggemma-300M-Q8_0.gguf";
const SIZE: u64 = 333_590_944;
/// Longer texts are cut. Most summaries are far shorter.
const TOKENS: u32 = 512;
/// A partial download that has not grown for this long belongs to a run that stopped.
const STALE: Duration = Duration::from_secs(600);
/// Writes the response body of `argv[1]` to the new file `argv[2]`.
const DOWNLOAD_PY: &str = "import shutil, sys, urllib.request
with urllib.request.urlopen(sys.argv[1], timeout=60) as response, open(sys.argv[2], 'xb') as out:
    shutil.copyfileobj(response, out, 1 << 20)";

/// The loaded model.
#[derive(Debug)]
pub(crate) struct Model {
    /// Declared first, because it must be freed before the backend.
    model: LlamaModel,
    backend: LlamaBackend,
}

impl Model {
    /// The model from the cache folder, downloaded first when it is not there. `None` when it
    /// cannot be had, after one line on stderr that says why.
    pub(crate) fn open() -> Option<Self> {
        let loaded = path().and_then(|path| {
            if !path.is_file() {
                download(&path)?;
            }
            Self::load(&path)
        });
        loaded
            .inspect_err(|e| eprintln!("xpd: {e:#}. Search uses keywords only."))
            .ok()
    }

    fn load(path: &Path) -> Result<Self> {
        let mut backend = LlamaBackend::init()?;
        backend.void_logs();
        let model = LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default())
            .with_context(|| format!("cannot load the embedding model {}", path.display()))?;
        Ok(Self { model, backend })
    }

    /// The vector of a search query.
    pub(crate) fn query(&self, query: &str) -> Result<Vec<f32>> {
        Ok(self
            .embed(&[format!("task: search result | query: {query}")])?
            .remove(0))
    }

    /// The vectors of index records, each given by its name and summary.
    pub(crate) fn documents(&self, records: &[(&str, &str)]) -> Result<Vec<Vec<f32>>> {
        let texts: Vec<String> = records
            .iter()
            .map(|(name, summary)| format!("title: {name} | text: {summary}"))
            .collect();
        self.embed(&texts)
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // llama.cpp's default of four threads. More threads slow a short query down many times
        // over on a machine that is busy with other work.
        let params = LlamaContextParams::default()
            .with_embeddings(true)
            .with_n_ctx(NonZeroU32::new(TOKENS))
            .with_n_batch(TOKENS)
            .with_n_ubatch(TOKENS);
        let mut context = self.model.new_context(&self.backend, params)?;
        let vocab = self.model.vocab();
        let limit = usize::try_from(TOKENS).expect("512 fits usize");
        let mut batch = LlamaBatch::new(limit, 1);
        texts
            .iter()
            .map(|text| {
                let mut tokens = vocab.tokenize(text.as_bytes(), true, false);
                if tokens.len() > limit {
                    // Keep the closing token that the tokenizer adds.
                    let last = tokens[tokens.len() - 1];
                    tokens.truncate(limit - 1);
                    tokens.push(last);
                }
                batch.clear();
                batch.add_sequence(&tokens, 0, false)?;
                context.clear_kv_cache();
                context.encode(&mut batch)?;
                Ok(normalised(&context.embeddings_seq_ith(0)?[..DIMENSIONS]))
            })
            .collect()
    }
}

fn normalised(vector: &[f32]) -> Vec<f32> {
    let norm = vector
        .iter()
        .map(|v| v * v)
        .sum::<f32>()
        .sqrt()
        .max(f32::EPSILON);
    vector.iter().map(|v| v / norm).collect()
}

/// Where the model file is kept, in the `models` folder of [`cache_folder`].
fn path() -> Result<PathBuf> {
    Ok(cache_folder()?.join("models").join(FILE))
}

/// The folder for files that xpd can make again: `$XDG_CACHE_HOME/xpd`, or else `xpd` in the
/// platform's cache folder.
pub(crate) fn cache_folder() -> Result<PathBuf> {
    let var = |name| env::var_os(name).filter(|value| !value.is_empty());
    let cache = if let Some(cache) = var("XDG_CACHE_HOME") {
        PathBuf::from(cache)
    } else if cfg!(windows) {
        PathBuf::from(var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?)
    } else {
        let home = PathBuf::from(var("HOME").context("HOME is not set")?);
        if cfg!(target_os = "macos") {
            home.join("Library").join("Caches")
        } else {
            home.join(".cache")
        }
    };
    Ok(cache.join("xpd"))
}

/// Downloads the model to a file of this process and renames it into place, so a stopped run
/// or a run beside it never leaves a partial model under the real name.
fn download(path: &Path) -> Result<()> {
    let folder = path.parent().context("the model path has a folder")?;
    fs::create_dir_all(folder)
        .with_context(|| format!("cannot create {} for the embedding model", folder.display()))?;
    remove_stale_parts(folder);
    let part = folder.join(format!("{FILE}.{}.part", std::process::id()));
    let python = venv::project_python();
    let result = Command::new(&python)
        .args(["-I", "-c", DOWNLOAD_PY, URL])
        .arg(&part)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "no Python to download the embedding model with ({})",
                python.display()
            )
        })
        .and_then(|child| {
            eprintln!(
                "xpd: downloading the {} MB embedding model once, to {}",
                SIZE / 1_000_000,
                path.display()
            );
            Ok(child.wait_with_output()?)
        })
        .and_then(|output| {
            let reason = String::from_utf8_lossy(&output.stderr);
            ensure!(
                output.status.success(),
                "cannot download the embedding model: {}",
                reason.trim().lines().last().unwrap_or("Python failed")
            );
            let size = fs::metadata(&part)?.len();
            ensure!(
                size == SIZE,
                "the embedding model download has {size} bytes, not {SIZE}"
            );
            fs::rename(&part, path)
                .with_context(|| format!("cannot move the embedding model to {}", path.display()))
        });
    if result.is_err() {
        // Python may have stopped before it created the file.
        let _ = fs::remove_file(&part);
    }
    result
}

/// Removes the partial downloads of runs that stopped before they finished.
fn remove_stale_parts(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let stopped = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .is_ok_and(|age| age > STALE)
            });
        if name.to_string_lossy().starts_with(FILE)
            && name.to_string_lossy().ends_with(".part")
            && stopped
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}
