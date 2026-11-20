//! Files of the fixtures folder, as `files` serves them.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use globset::GlobBuilder;
use otty_plugin_manifest::{JsonPointer, Permissions};
use serde_json::{Map, Value};

use crate::files::{Batch, FileInfo};
use crate::test_host::{HostState, not_allowed};
use crate::{HostError, JsonWithKeys};

/// The most files `list` gives (spec "Host operations").
const MOST_FILES: usize = 100_000;

/// The most bytes `read` gives.
const READ_CAP: u32 = 4 * 1024 * 1024;

/// The files under `home` whose paths match `glob` and that the part may
/// read, read with its keys replaced, or query.
pub(super) fn list(
    home: &Path,
    permissions: &Permissions,
    glob: &str,
) -> Result<Vec<FileInfo>, HostError> {
    let matcher = GlobBuilder::new(glob)
        .literal_separator(true)
        .build()
        .map_err(|error| HostError::Failed(format!("`{glob}`: {error}")))?
        .compile_matcher();

    let mut paths = Vec::new();
    collect_files(home, &mut paths);

    let found = paths
        .into_iter()
        .filter(|path| matcher.is_match(path))
        .filter(|path| {
            permissions.may_read(path)
                || permissions.may_read_with_keys(path)
                || permissions.may_query(path)
        })
        .take(MOST_FILES)
        .filter_map(|path| file_info(&path))
        .collect();

    Ok(found)
}

/// The bytes of `path`, at most `max_bytes`.
pub(super) fn read(
    permissions: &Permissions,
    path: &str,
    max_bytes: u32,
) -> Result<Vec<u8>, HostError> {
    let path = PathBuf::from(path);
    if !permissions.may_read(&path) {
        return Err(not_allowed(&path));
    }

    let limit = max_bytes.min(READ_CAP);
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .and_then(|file| {
            file.take(u64::from(limit) + 1).read_to_end(&mut bytes)
        })
        .map_err(|error| io_error(&error, &path))?;

    if bytes.len() > limit as usize {
        return Err(HostError::TooBig(path.display().to_string()));
    }

    Ok(bytes)
}

/// Complete lines of `path` from `offset`, about `max_bytes` of them, only
/// those holding one of `markers`.
pub(super) fn read_lines(
    permissions: &Permissions,
    path: &str,
    offset: u64,
    max_bytes: u32,
    markers: &[Vec<u8>],
) -> Result<Batch, HostError> {
    let path = PathBuf::from(path);
    if !permissions.may_read(&path) {
        return Err(not_allowed(&path));
    }

    let bytes = fs::read(&path).map_err(|error| io_error(&error, &path))?;
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(bytes.len());

    let mut position = start;
    let mut lines = Vec::new();
    let mut skipped = 0;
    while position < bytes.len() {
        let rest = &bytes[position..];
        let length = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(rest.len(), |newline| newline + 1);
        let read_so_far = position - start;
        if read_so_far > 0 && read_so_far + length > max_bytes as usize {
            break;
        }

        let line = &rest[..length];
        if has_marker(line, markers) {
            lines.extend_from_slice(line);
        } else {
            skipped += 1;
        }
        position += length;
    }

    Ok(Batch {
        lines,
        next_offset: position as u64,
        skipped,
        at_end: position >= bytes.len(),
    })
}

/// The JSON key file `path` with the key fields the part declares
/// replaced by `{"$key": n}`.
pub(super) fn read_json_with_keys(
    state: &mut HostState,
    permissions: &Permissions,
    path: &str,
) -> Result<JsonWithKeys, HostError> {
    let path = PathBuf::from(path);
    if !permissions.may_read_with_keys(&path) {
        return Err(not_allowed(&path));
    }

    let text =
        fs::read_to_string(&path).map_err(|error| io_error(&error, &path))?;
    let mut document: Value = serde_json::from_str(&text).map_err(|_| {
        HostError::Failed(format!("{} is not JSON", path.display()))
    })?;

    let fields = permissions.key_fields(&path);
    let mut keys = Vec::new();
    replace_keys(&mut document, "", &fields, &mut |value| {
        keys.push(state.add_key(value));

        keys.len() - 1
    });

    Ok(JsonWithKeys::new(document.to_string(), keys))
}

/// An I/O error of `path` in the words of a host error.
pub(super) fn io_error(error: &io::Error, path: &Path) -> HostError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            HostError::NotFound(path.display().to_string())
        },
        _ => HostError::Failed(format!("{}: {error}", path.display())),
    }
}

/// Every file under `folder`, folders walked into.
fn collect_files(folder: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, paths);
        } else {
            paths.push(path);
        }
    }
}

fn file_info(path: &Path) -> Option<FileInfo> {
    let metadata = fs::metadata(path).ok()?;
    let changed_ms = metadata
        .modified()
        .ok()
        .and_then(|changed| changed.duration_since(UNIX_EPOCH).ok())
        .and_then(|since_epoch| i64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or_default();

    Some(FileInfo {
        path: path.display().to_string(),
        size: metadata.len(),
        changed_ms,
    })
}

fn has_marker(line: &[u8], markers: &[Vec<u8>]) -> bool {
    markers.is_empty()
        || markers.iter().any(|marker| {
            marker.is_empty()
                || line
                    .windows(marker.len())
                    .any(|part| part == marker.as_slice())
        })
}

/// Replaces each text value of `value` at one of `fields` by `{"$key":
/// n}`, where `n` is what `add_key` gives for the text.
pub(super) fn replace_keys(
    value: &mut Value,
    pointer: &str,
    fields: &[JsonPointer],
    add_key: &mut impl FnMut(String) -> usize,
) {
    match value {
        Value::String(text)
            if fields.iter().any(|field| field.matches(pointer)) =>
        {
            let index = add_key(std::mem::take(text));
            let mut handle = Map::new();
            handle.insert("$key".to_string(), Value::from(index));

            *value = Value::Object(handle);
        },
        Value::Object(members) => {
            for (name, member) in members.iter_mut() {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                replace_keys(
                    member,
                    &format!("{pointer}/{escaped}"),
                    fields,
                    add_key,
                );
            }
        },
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                replace_keys(
                    item,
                    &format!("{pointer}/{index}"),
                    fields,
                    add_key,
                );
            }
        },
        _ => {},
    }
}
