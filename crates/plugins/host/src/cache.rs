//! The disk cache of compiled plugins (spec "Engine limits": 0.5 s to
//! compile, 0.3 ms to load from the cache). Only this crate writes the
//! cache folder.

use std::hash::Hasher;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use wasmtime::Engine;
use wasmtime::component::Component;

/// The cache folder, such as `~/.cache/otty/plugins/`, with one file per
/// plugin `.wasm` and engine: `<sha256 of the .wasm>-<engine hash>.cwasm`.
#[derive(Debug, Clone)]
pub(crate) struct CompiledCache {
    folder: PathBuf,
}

impl CompiledCache {
    pub(crate) fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// The component of `wasm`, read from the cache when this engine
    /// compiled it before, else compiled now and kept. A broken or foreign
    /// cache file is compiled again and replaced.
    pub(crate) fn load(
        &self,
        engine: &Engine,
        wasm: &[u8],
    ) -> wasmtime::Result<(Component, LoadKind)> {
        let path = self.folder.join(file_name(engine, wasm));

        if path.is_file() {
            // SAFETY: `deserialize_file` runs the machine code in the file,
            // so the file must come from wasmtime. Only this crate writes
            // the cache folder, under names made from the SHA-256 of the
            // `.wasm` and of the engine's settings, and always through a
            // temporary file renamed into place, so a file is never read
            // half-written. wasmtime checks the file's header and refuses
            // output of another version or other settings; that error, or
            // any other, leads to a fresh compile below.
            let cached = unsafe { Component::deserialize_file(engine, &path) };
            if let Ok(component) = cached {
                return Ok((component, LoadKind::Cached));
            }
        }

        let compiled = engine.precompile_component(wasm)?;
        // SAFETY: the bytes were made by this engine just now.
        let component = unsafe { Component::deserialize(engine, &compiled)? };

        if let Err(error) = write_atomically(&path, &compiled) {
            log::warn!(
                "a compiled plugin could not be kept in the cache: {error}"
            );
        }

        Ok((component, LoadKind::Compiled))
    }

    /// Removes the compiled copy of the `.wasm` whose SHA-256 is
    /// `wasm_sha256` (lowercase hex), made by this engine. A copy that is
    /// not there is not an error.
    pub(crate) fn remove(
        &self,
        engine: &Engine,
        wasm_sha256: &str,
    ) -> std::io::Result<()> {
        let path = self.folder.join(file_name_of_hash(engine, wasm_sha256));

        match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error)
            },
            _ => Ok(()),
        }
    }
}

/// How a component was loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    /// Compiled with Cranelift and written to the cache.
    Compiled,
    /// Read from the cache.
    Cached,
}

/// A `Hasher` that feeds SHA-256, so the engine's compatibility hash gives
/// the same name on every run, unlike the standard hasher.
struct Sha256Hasher {
    sha: Sha256,
}

impl Hasher for Sha256Hasher {
    fn finish(&self) -> u64 {
        let digest = self.sha.clone().finalize();
        let mut first = [0_u8; 8];
        first.copy_from_slice(&digest[..8]);

        u64::from_le_bytes(first)
    }

    fn write(&mut self, bytes: &[u8]) {
        self.sha.update(bytes);
    }
}

/// The cache file name of `wasm` compiled by `engine`.
fn file_name(engine: &Engine, wasm: &[u8]) -> String {
    file_name_of_hash(engine, &hex(&Sha256::digest(wasm)))
}

/// The cache file name of the `.wasm` whose SHA-256 is `wasm_hash`
/// compiled by `engine`.
fn file_name_of_hash(engine: &Engine, wasm_hash: &str) -> String {
    let mut hasher = Sha256Hasher { sha: Sha256::new() };
    std::hash::Hash::hash(&engine.precompile_compatibility_hash(), &mut hasher);
    let engine_hash = hex(&hasher.sha.finalize()[..8]);

    format!("{wasm_hash}-{engine_hash}.cwasm")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Writes `bytes` to a temporary file next to `path`, then renames it
/// into place.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }

    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)?;

    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, fixture};

    fn engine() -> Engine {
        Engine::default()
    }

    fn wasm() -> Vec<u8> {
        std::fs::read(fixture("minimal")).expect("the fixture")
    }

    fn cache_files(folder: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(folder)
            .expect("the cache folder")
            .map(|entry| entry.expect("an entry").path())
            .collect()
    }

    #[test]
    fn a_second_load_comes_from_the_cache() {
        let folder = TempDir::new();
        let cache = CompiledCache::new(folder.path().join("plugins"));
        let engine = engine();

        let (_, first) = cache.load(&engine, &wasm()).expect("compiles");
        let (_, second) = cache.load(&engine, &wasm()).expect("loads");

        assert_eq!(first, LoadKind::Compiled);
        assert_eq!(second, LoadKind::Cached);
        assert_eq!(cache_files(&folder.path().join("plugins")).len(), 1);
    }

    #[test]
    fn a_damaged_cache_file_is_compiled_again() {
        let folder = TempDir::new();
        let cache = CompiledCache::new(folder.path().to_path_buf());
        let engine = engine();
        cache.load(&engine, &wasm()).expect("compiles");
        let [cached] = cache_files(folder.path()).try_into().expect("one file");
        std::fs::write(&cached, b"not a compiled plugin").expect("damage");

        let (_, kind) = cache.load(&engine, &wasm()).expect("compiles again");
        let (_, again) = cache.load(&engine, &wasm()).expect("loads");

        assert_eq!(kind, LoadKind::Compiled);
        assert_eq!(again, LoadKind::Cached);
    }

    #[test]
    fn a_compiled_copy_is_removed_by_the_hash_of_its_wasm() {
        let folder = TempDir::new();
        let cache = CompiledCache::new(folder.path().to_path_buf());
        let engine = engine();
        cache.load(&engine, &wasm()).expect("compiles");

        cache
            .remove(&engine, &hex(&Sha256::digest(wasm())))
            .expect("removed");
        let again = cache.remove(&engine, &hex(&Sha256::digest(wasm())));

        assert!(cache_files(folder.path()).is_empty());
        assert!(again.is_ok(), "a missing copy is not an error");
    }

    #[test]
    fn the_cache_file_is_named_by_the_wasm_and_the_engine() {
        let engine = engine();
        let wasm = wasm();

        let name = file_name(&engine, &wasm);

        assert_eq!(name, file_name(&engine, &wasm));
        assert!(name.starts_with(&hex(&Sha256::digest(&wasm))));
        assert!(name.ends_with(".cwasm"));
        assert_ne!(name, file_name(&engine, b"other bytes"));
    }
}
