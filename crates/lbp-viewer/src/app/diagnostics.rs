//! Authored shared private diagnostic JSON envelope helpers and bounded writer.
//! Metadata only; never reads an original source or invokes a resource parser.
//! Policy and verification: docs/material-export.md and docs/texture-export.md.
use lbp_catalog::attachment::LocationKey;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn reason(error: &str) -> Option<&'static str> {
    if error.is_empty() {
        return None;
    }
    let e = error.to_ascii_lowercase();
    Some(if e.contains("cancel") {
        "cancelled"
    } else if e.contains("protected") || e.contains("encrypted") {
        "blocked_protected"
    } else if e.contains("unsupported") {
        "unsupported_layout"
    } else if e.contains("budget") || e.contains("limit") {
        "budget_exceeded"
    } else if e.contains("changed") {
        "source_changed"
    } else {
        "inspection_failed"
    })
}
pub(super) fn stage(metadata: Option<Value>, requested: bool, error: &str) -> Value {
    let status = if metadata.is_some() {
        "available"
    } else if !error.is_empty() {
        match reason(error) {
            Some("cancelled") => "cancelled",
            Some("blocked_protected") => "blocked",
            Some("unsupported_layout") => "unsupported",
            _ => "failed",
        }
    } else if requested {
        "not_available"
    } else {
        "not_requested"
    };
    json!({"status":status,"requested_in_current_mode":requested,
           "error_kind":reason(error),"metadata":metadata})
}
pub(super) fn location(key: &LocationKey) -> Result<Value, String> {
    if key.path.contains(['\\', ':', '\0'])
        || key
            .path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err("Diagnostic source path must be relative and normalized".into());
    }
    Ok(json!({"source_path":key.path,"offset":key.offset,"size":key.size}))
}
fn private_root() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("Workspace absent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let expected = root.join("local-analysis");
    let allowed = expected
        .canonicalize()
        .map_err(|_| "Ignored local-analysis directory must exist")?;
    if allowed != expected {
        return Err("local-analysis must be an ordinary workspace directory".into());
    }
    Ok(allowed)
}
pub(super) fn checked_path(path: &Path) -> Result<PathBuf, String> {
    let allowed = private_root()?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|_| "Export parent directory must already exist")?;
    let destination = parent.join(path.file_name().ok_or("Export needs a filename")?);
    if !parent.starts_with(allowed)
        || destination.extension().is_none_or(|s| s != "json")
        || destination.exists()
    {
        return Err(
            "Choose a new .json file inside ignored local-analysis; overwrite is refused".into(),
        );
    }
    Ok(destination)
}
struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_JSON_BYTES)
        {
            return Err(std::io::Error::other(
                "Diagnostic JSON byte budget exceeded",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(super) fn encode(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = BoundedJson(Vec::new());
    serde_json::to_writer_pretty(&mut bytes, value).map_err(|e| e.to_string())?;
    bytes.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(bytes.0)
}
pub(super) fn save(value: &Value, path: &Path) -> Result<(), String> {
    let bytes = encode(value)?;
    let destination = checked_path(path)?;
    // Exclusive create also refuses existing/raced leaf symlinks and hard links.
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.flush())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct OwnedDir(PathBuf);
    impl OwnedDir {
        fn new() -> Self {
            // A fresh clone (CI) has no ignored local-analysis directory yet.
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            std::fs::create_dir_all(workspace.join("local-analysis")).unwrap();
            let path = private_root().unwrap().join(format!(
                "test-diagnostic-export-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for OwnedDir {
        fn drop(&mut self) {
            let root = private_root().unwrap();
            let target = self.0.canonicalize().unwrap();
            assert!(target.starts_with(root) && target == self.0);
            std::fs::remove_dir_all(target).unwrap();
        }
    }
    #[test]
    fn exclusive_private_output_rejects_existing_escape_extension_and_missing_parent() {
        let dir = OwnedDir::new();
        let path = dir.0.join("result.json");
        let value = json!({"schema_version":1,"scope":"authored_synthetic_test_metadata"});
        save(&value, &path).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(save(&value, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(checked_path(Path::new("docs/diagnostic-export.json")).is_err());
        assert!(checked_path(&dir.0.join("../../docs/diagnostic-export.json")).is_err());
        for p in ["missing/result.json", "asset.bin", "result.json:alternate"] {
            assert!(checked_path(&dir.0.join(p)).is_err());
        }
        let second = dir.0.join("second.json");
        save(&value, &second).unwrap();
        assert_eq!(std::fs::read(second).unwrap(), before);
    }
    #[test]
    fn bounded_output_precedes_file_creation_and_location_and_errors_exclude_host_paths() {
        let mut writer = BoundedJson(vec![0; MAX_JSON_BYTES - 1]);
        writer.write_all(b"x").unwrap();
        assert!(writer.write_all(b"y").is_err());
        assert_eq!(writer.0.len(), MAX_JSON_BYTES);
        let dir = OwnedDir::new();
        let path = dir.0.join("too-large.json");
        assert!(save(&json!("x".repeat(MAX_JSON_BYTES)), &path).is_err());
        assert!(!path.exists());
        let state = stage(
            None,
            true,
            "Failed reading C:\\private\\game-data\\file.bin",
        );
        assert!(!String::from_utf8(encode(&state).unwrap())
            .unwrap()
            .contains("private"));
        assert_eq!(state["error_kind"], "inspection_failed");
        for p in [
            "C:/secret/file",
            "../file",
            "/absolute/file",
            "USRDIR/../file",
            "USRDIR\\file",
            "",
        ] {
            assert!(location(&LocationKey {
                path: p.into(),
                offset: 0,
                size: 0
            })
            .is_err());
        }
    }
}
