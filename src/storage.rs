use crate::model::Project;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub fn backup_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".bak");
    p.into()
}

pub fn load(path: &Path) -> Result<Project, String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Kunde inte öppna '{}': {e}", path.display()))?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Err("Projektfilen är större än 32 MB.".into());
    }
    let bytes = fs::read(path).map_err(|e| format!("Kunde inte läsa projektet: {e}"))?;
    let project: Project = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Projektfilen är inte ett giltigt Cable Management-projekt: {e}"))?;
    project.validate()?;
    Ok(project)
}

pub fn save(path: &Path, project: &Project) -> Result<(), String> {
    project.validate()?;
    let bytes = serde_json::to_vec_pretty(project)
        .map_err(|e| format!("Kunde inte skriva projektet: {e}"))?;
    let mut temp = path.as_os_str().to_owned();
    temp.push(format!(".{}.tmp", std::process::id()));
    let temp = PathBuf::from(temp);
    let write = || -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| format!("Kunde inte spara i '{}': {e}", path.display()))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Kunde inte skriva projektfilen: {e}"))?;
        if path.exists() {
            // A backup is copied before replacement; failure preserves the original.
            fs::copy(path, backup_path(path))
                .map_err(|e| format!("Kunde inte spara säkerhetskopian: {e}"))?;
        }
        replace_file(&temp, path).map_err(|e| format!("Kunde inte spara projektet: {e}"))?;
        Ok(())
    };
    let result = write();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH; no remove-then-rename gap.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 1 | 8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn recovery_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".local/share"))
                .unwrap_or_else(std::env::temp_dir)
        });
    base.join("CableManagement").join("recovery.cable.json")
}

pub fn save_recovery(project: &Project) -> Result<(), String> {
    let path = recovery_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Kunde inte spara återställning: {e}"))?;
    }
    save(&path, project)
}
