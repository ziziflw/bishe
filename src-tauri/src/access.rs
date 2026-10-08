//! Windows-only, fail-closed external metadata access. No file contents or writes.
use crate::{
    error::{AppError, Result},
    model::Grant,
};
use std::os::windows::{
    fs::{MetadataExt, OpenOptionsExt},
    io::AsRawHandle,
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
};

pub fn now() -> i64 {
    millis(SystemTime::now())
}
fn millis(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
pub fn path_key(path: &str) -> String {
    path.to_lowercase()
}

/// Reject ambiguous Windows spellings before touching the external filesystem.
pub fn validate_path(path: &Path) -> Result<PathBuf> {
    let raw = path.to_str().ok_or_else(AppError::invalid)?;
    if raw.len() < 3
        || raw.len() > 240
        || !raw.as_bytes()[0].is_ascii_alphabetic()
        || &raw.as_bytes()[1..3] != b":\\"
        || raw.contains('/')
        || raw.chars().any(|c| c.is_control())
    {
        return Err(AppError::invalid());
    }
    let rest = raw[3..].trim_end_matches('\\');
    if !rest.is_empty() {
        for part in rest.split('\\') {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part.contains([':', '*', '?', '"', '<', '>', '|'])
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit())
            {
                return Err(AppError::invalid());
            }
        }
    }
    let mut clean = raw[..3].to_string();
    clean.push_str(rest);
    Ok(PathBuf::from(clean))
}
fn contains(root: &str, candidate: &str) -> bool {
    Path::new(&path_key(candidate)).starts_with(Path::new(&path_key(root)))
}
fn identity(file: &File) -> Result<String> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: live File owns the handle, info is initialized and writable.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(format!(
        "{:08x}:{:08x}{:08x}:{:08x}{:08x}",
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
        info.ftCreationTime.dwHighDateTime,
        info.ftCreationTime.dwLowDateTime
    ))
}
pub struct Snapshot {
    pub path: String,
    pub identity: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: i64,
    // Hold all ancestors and the target until the operation/transaction completes.
    _handles: Vec<(String, File)>,
}
impl Snapshot {
    pub fn open_selected(path: &Path, directory: bool) -> Result<Self> {
        let clean = validate_path(path)?;
        let root: Vec<u16> = clean.to_str().ok_or_else(AppError::invalid)?[..3]
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: root is a NUL-terminated drive root, never an arbitrary device path.
        let drive_type = unsafe { GetDriveTypeW(root.as_ptr()) };
        if !matches!(drive_type, 2 | 3) {
            return Err(AppError::new(
                "DRIVE",
                "本版仅支持本地固定或可移动磁盘，不接受网络映射盘或未知设备。",
            ));
        }
        let mut ancestors: Vec<_> = clean.ancestors().map(Path::to_path_buf).collect();
        ancestors.reverse();
        let mut handles = Vec::new();
        for (i, item) in ancestors.iter().enumerate() {
            let is_dir = i + 1 < ancestors.len() || directory;
            let mut options = OpenOptions::new();
            options
                .access_mode(if is_dir { 0x80 } else { 0x80000000 })
                .share_mode(1) // FILE_SHARE_READ only: deny replacement/write while held.
                .custom_flags(0x00200000 | 0x02000000); // OPEN_REPARSE_POINT | BACKUP_SEMANTICS
            let file = options.open(item)?;
            let meta = file.metadata()?;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(AppError::new(
                    "LINK",
                    "为确保授权范围安全，本版拒绝符号链接、junction 及其他 reparse point。",
                ));
            }
            if meta.is_dir() != is_dir || (!is_dir && !meta.is_file()) {
                return Err(AppError::invalid());
            }
            handles.push((item.to_string_lossy().into_owned(), file));
        }
        let file = &handles.last().ok_or_else(AppError::invalid)?.1;
        let meta = file.metadata()?;
        let size = i64::try_from(meta.len()).map_err(|_| AppError::invalid())?;
        Ok(Self {
            path: clean.to_str().ok_or_else(AppError::invalid)?.into(),
            identity: identity(file)?,
            name: clean
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .into(),
            extension: clean
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_lowercase(),
            size,
            modified_at: millis(meta.modified()?),
            _handles: handles,
        })
    }
}
/// Check lexical scope before any external access, then validate the live root identity.
pub fn authorized_snapshot(
    path: &Path,
    expected_identity: &str,
    grants: &[Grant],
) -> Result<Snapshot> {
    let clean = validate_path(path)?;
    let candidate = clean.to_str().ok_or_else(AppError::invalid)?;
    let matching: Vec<_> = grants
        .iter()
        .filter(|g| {
            g.active
                && if g.kind == "FILE" {
                    path_key(&g.path) == path_key(candidate) && g.identity == expected_identity
                } else {
                    contains(&g.path, candidate)
                }
        })
        .collect();
    if matching.is_empty() {
        return Err(AppError::denied());
    }
    // Validate DIRECTORY authorization roots before probing a possibly missing target.
    let mut root_locks = Vec::new();
    let mut valid = false;
    for grant in matching {
        if grant.kind == "FILE" {
            valid = true;
        } else if let Some(candidate_root) = clean.ancestors().find(|p| {
            p.to_str()
                .is_some_and(|s| path_key(s) == path_key(&grant.path))
        }) {
            if let Ok(root) = Snapshot::open_selected(candidate_root, true) {
                if root.identity == grant.identity {
                    valid = true;
                    root_locks.push(root);
                }
            }
        }
    }
    if !valid {
        return Err(AppError::denied());
    }
    let snapshot = Snapshot::open_selected(&clean, false)?;
    if snapshot.identity != expected_identity {
        return Err(AppError::new(
            "CHANGED",
            "此路径现在指向不同文件，原标签记录已保留。",
        ));
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ambiguous_paths_and_component_prefixes() {
        for p in [
            "relative.txt",
            "C:\\ok\\..\\secret",
            "C:\\ok\\.\\x",
            "C:\\a.txt:stream",
            "\\\\server\\share\\x",
            "\\\\?\\C:\\x",
            "C:\\ok \\x",
            "C:\\NUL.txt",
            "C:\\a//x",
        ] {
            assert!(validate_path(Path::new(p)).is_err(), "{p}");
        }
        assert!(!contains("C:\\allowed", "C:\\allowed-other\\x"));
        assert!(contains("C:\\Allowed", "c:\\allowed\\x"));
    }
}
