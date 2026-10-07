//! Read-only filesystem catalog. Never follows nested symbolic links/reparse points.
use lbp_core::{Error, ErrorKind, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct SourceFile {
    pub path: String,
    pub size: u64,
}
#[derive(Debug, Serialize)]
pub struct SkippedEntry {
    pub path: String,
    pub reason: &'static str,
}
pub struct Source {
    pub selected_root: PathBuf,
    pub game_root: PathBuf,
    pub files: Vec<SourceFile>,
    pub skipped: Vec<SkippedEntry>,
    pub directory_count: usize,
}

fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
pub fn relative(path: &Path, root: &Path) -> Result<String> {
    path.strip_prefix(root)
        .map_err(|_| Error::malformed("filesystem path escaped root"))?
        .to_str()
        .map(|s| s.replace('\\', "/"))
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, "non-UTF8 filesystem path"))
}

fn matching_directory(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    let mut matches = Vec::new();
    for item in fs::read_dir(root)? {
        let item = item?;
        if item
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
        {
            let metadata = fs::symlink_metadata(item.path())?;
            if is_link(&metadata) {
                return Err(Error::malformed(format!(
                    "required directory {name} is a symlink/reparse point"
                )));
            }
            if metadata.is_dir() {
                matches.push(item.path());
            }
        }
    }
    if matches.len() > 1 {
        return Err(Error::malformed(format!(
            "case-ambiguous {name} directories"
        )));
    }
    Ok(matches.pop())
}

impl Source {
    pub fn discover(input: &Path, max_entries: usize) -> Result<Self> {
        Self::discover_checked(input, max_entries, || Ok(()))
    }
    pub fn discover_checked(
        input: &Path,
        max_entries: usize,
        check: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        check()?;
        let selected_root = fs::canonicalize(input)?;
        if !selected_root.is_dir() {
            return Err(Error::malformed(
                "--game-dir must be an extracted directory",
            ));
        }
        let game_root = matching_directory(&selected_root, "PS3_GAME")?
            .unwrap_or_else(|| selected_root.clone());
        if matching_directory(&game_root, "USRDIR")?.is_none() {
            return Err(Error::malformed(
                "expected disc root or PS3_GAME root containing USRDIR",
            ));
        }
        let mut source = Self {
            selected_root,
            game_root: game_root.clone(),
            files: Vec::new(),
            skipped: Vec::new(),
            directory_count: 1,
        };
        let mut pending = vec![game_root];
        let mut total = 0_usize;
        while let Some(directory) = pending.pop() {
            for item in fs::read_dir(directory)? {
                check()?;
                let item = item?;
                total = total
                    .checked_add(1)
                    .ok_or_else(|| Error::limit("filesystem count overflow"))?;
                if total > max_entries {
                    return Err(Error::limit("filesystem entry budget exceeded"));
                }
                let path = item.path();
                let name = relative(&path, &source.game_root)?;
                let metadata = fs::symlink_metadata(&path)?;
                if is_link(&metadata) {
                    source.skipped.push(SkippedEntry {
                        path: name,
                        reason: "symlink_or_reparse_point_not_followed",
                    });
                } else if metadata.is_dir() {
                    source.directory_count += 1;
                    pending.push(path);
                } else if metadata.is_file() {
                    source.files.push(SourceFile {
                        path: name,
                        size: metadata.len(),
                    });
                } else {
                    source.skipped.push(SkippedEntry {
                        path: name,
                        reason: "non_regular_file_not_read",
                    });
                }
            }
        }
        source.files.sort_by(|a, b| a.path.cmp(&b.path));
        source.skipped.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(source)
    }
    pub fn required(&self, name: &str) -> Result<&SourceFile> {
        let matches: Vec<_> = self
            .files
            .iter()
            .filter(|f| f.path.eq_ignore_ascii_case(name))
            .collect();
        match matches.as_slice() {
            [file] => Ok(file),
            [] => Err(Error::malformed(format!(
                "required input {name} absent (links are not followed)"
            ))),
            _ => Err(Error::malformed(format!(
                "required input {name} is case-ambiguous"
            ))),
        }
    }
    pub fn open(&self, file: &SourceFile) -> Result<fs::File> {
        if !safe_relative(&file.path) {
            return Err(Error::malformed("unsafe source-relative path"));
        }
        let mut checked = self.game_root.clone();
        if is_link(&fs::symlink_metadata(&checked)?) {
            return Err(Error::malformed("source root became a link"));
        }
        for part in file.path.split('/') {
            checked.push(part);
            if is_link(&fs::symlink_metadata(&checked)?) {
                return Err(Error::malformed(
                    "source path contains symlink/reparse point",
                ));
            }
        }
        let path = self.game_root.join(&file.path);
        let metadata = fs::symlink_metadata(&path)?;
        if is_link(&metadata) || !metadata.is_file() {
            return Err(Error::malformed("input changed or became non-regular"));
        }
        let canonical = fs::canonicalize(&path)?;
        if !path_within(&canonical, &self.game_root) {
            return Err(Error::malformed("input resolved outside game source"));
        }
        let handle = fs::File::open(canonical)?;
        if handle.metadata()?.len() != file.size {
            return Err(Error::malformed("source file changed size"));
        }
        Ok(handle)
    }
}

/// Strict PS3_GAME-relative filesystem key, unlike MAP paths which are USRDIR-relative.
pub fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Pure lexical MAP-path validation; never trusts a MAP path as an OS path.
pub fn map_relative(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    if path.is_empty() || path.starts_with('/') || path.contains(':') || path.contains('\0') {
        return None;
    }
    let mut components = Vec::new();
    for part in path.split('/') {
        if part == ".." {
            return None;
        }
        if !part.is_empty() && part != "." {
            components.push(part);
        }
    }
    if components.is_empty() {
        return None;
    }
    Some(format!("USRDIR/{}", components.join("/")))
}

pub fn path_within(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        path.to_string_lossy().to_ascii_lowercase().starts_with(
            &(root
                .to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .to_ascii_lowercase()
                + "\\"),
        ) || path
            .to_string_lossy()
            .eq_ignore_ascii_case(&root.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

pub fn check_output(path: &Path, source: &Source) -> Result<PathBuf> {
    if path.exists() {
        return Err(Error::malformed(
            "JSON output already exists; choose a new filename",
        ));
    }
    let filename = path
        .file_name()
        .ok_or_else(|| Error::malformed("JSON output needs a filename"))?;
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::malformed("JSON output path may not contain '..'"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent)?;
    let destination = parent.join(filename);
    if path_within(&destination, &source.selected_root)
        || path_within(&destination, &source.game_root)
    {
        return Err(Error::malformed(
            "JSON output must be outside selected game data",
        ));
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_checks_cancellation_while_enumerating() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("PS3_GAME/USRDIR")).unwrap();
        fs::write(temp.path().join("PS3_GAME/USRDIR/authored"), b"x").unwrap();
        let calls = std::cell::Cell::new(0);
        let result = Source::discover_checked(temp.path(), 20, || {
            calls.set(calls.get() + 1);
            if calls.get() >= 3 {
                Err(Error::malformed("cancelled"))
            } else {
                Ok(())
            }
        });
        assert!(result.err().unwrap().message.contains("cancelled"));
    }
    #[cfg(windows)]
    #[test]
    fn open_rejects_new_ancestor_junction_even_with_target_inside_source() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("PS3_GAME");
        let nested = game.join("USRDIR/nested");
        let actual = game.join("USRDIR/actual");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("authored"), b"x").unwrap();
        let source = Source::discover(temp.path(), 20).unwrap();
        fs::rename(&nested, &actual).unwrap(); // authored temporary directory only
        let status=std::process::Command::new("powershell.exe")
            .args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:LBP_TEST_LINK -Target $env:LBP_TEST_TARGET | Out-Null"])
            .env("LBP_TEST_LINK",&nested).env("LBP_TEST_TARGET",&actual).status().unwrap();
        assert!(status.success());
        assert!(source
            .open(&source.files[0])
            .err()
            .unwrap()
            .message
            .contains("reparse"));
        fs::remove_dir(nested).unwrap(); // remove only the authored junction
        assert_eq!(fs::read(actual.join("authored")).unwrap(), b"x");
    }
    #[cfg(windows)]
    #[test]
    fn nested_junction_is_not_followed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("disc");
        let outside = temp.path().join("outside");
        fs::create_dir_all(root.join("PS3_GAME/USRDIR")).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("sentinel"), b"outside").unwrap();
        let link = root.join("PS3_GAME/USRDIR/junction");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:LBP_TEST_LINK -Target $env:LBP_TEST_TARGET | Out-Null"])
            .env("LBP_TEST_LINK", &link).env("LBP_TEST_TARGET", &outside)
            .status().unwrap();
        assert!(status.success());
        let source = Source::discover(&root, 20).unwrap();
        assert!(source.files.is_empty());
        assert_eq!(source.skipped.len(), 1);
        assert_eq!(source.skipped[0].path, "USRDIR/junction");
        assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"outside");
        fs::remove_dir(link).unwrap(); // Remove the authored junction only, not its target.
    }
    #[test]
    fn map_paths_do_not_become_os_paths() {
        for bad in [
            "",
            "..",
            "a/../b",
            "/outside",
            "C:\\outside",
            "\\\\server\\file",
            "a\0b",
        ] {
            assert!(map_relative(bad).is_none(), "{bad:?}");
        }
        assert_eq!(
            map_relative("./foo\\bar.tex").as_deref(),
            Some("USRDIR/foo/bar.tex")
        );
    }
    #[test]
    fn root_containment_checks_component_boundary() {
        let temp = tempfile::tempdir().unwrap();
        assert!(path_within(&temp.path().join("inside"), temp.path()));
        assert!(path_within(temp.path(), temp.path()));
        let sibling = temp.path().with_file_name(format!(
            "{}-other",
            temp.path().file_name().unwrap().to_string_lossy()
        ));
        assert!(!path_within(&sibling, temp.path()));
    }
}
