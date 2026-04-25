use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Installation prefix: `$D_PREFIX` or `~/.d`.
pub fn prefix() -> PathBuf {
    if let Ok(p) = std::env::var("D_PREFIX") {
        return PathBuf::from(p);
    }
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".d")
}

/// The bin directory where the active `deno` symlink lives.
pub fn bin_dir() -> PathBuf {
    prefix().join("bin")
}

/// Activate a cached version by creating/updating a symlink.
pub fn activate(tag: &str) -> Result<()> {
    let bin = bin_dir();
    fs::create_dir_all(&bin).context("Failed to create bin directory")?;

    let deno_src = crate::cache::deno_binary(tag);

    #[cfg(target_os = "windows")]
    let link_path = bin.join("deno.exe");
    #[cfg(not(target_os = "windows"))]
    let link_path = bin.join("deno");

    if link_path.exists() || link_path.symlink_metadata().is_ok() {
        fs::remove_file(&link_path).ok();
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&deno_src, &link_path).with_context(|| {
        format!(
            "Failed to create symlink {} -> {}",
            link_path.display(),
            deno_src.display()
        )
    })?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&deno_src, &link_path).with_context(|| {
        format!(
            "Failed to create symlink {} -> {}",
            link_path.display(),
            deno_src.display()
        )
    })?;

    let marker = prefix().join(".active");
    fs::write(&marker, tag).context("Failed to write active version marker")?;

    Ok(())
}

/// Read the currently active version from the marker file.
pub fn active_version() -> Option<String> {
    let marker = prefix().join(".active");
    fs::read_to_string(marker)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Remove the active deno symlink (does not remove cache).
pub fn uninstall() -> Result<()> {
    let bin = bin_dir();

    #[cfg(target_os = "windows")]
    let link_path = bin.join("deno.exe");
    #[cfg(not(target_os = "windows"))]
    let link_path = bin.join("deno");

    if link_path.exists() || link_path.symlink_metadata().is_ok() {
        fs::remove_file(&link_path).context("Failed to remove deno symlink")?;
        println!("Removed active Deno installation.");
    } else {
        println!("No active Deno installation found.");
    }

    let marker = prefix().join(".active");
    fs::remove_file(marker).ok();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_prefix<F: FnOnce(&std::path::Path)>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("D_PREFIX", dir.path());
        std::env::remove_var("D_CACHE_DIR");
        f(dir.path());
        std::env::remove_var("D_PREFIX");
    }

    #[test]
    fn prefix_respects_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("D_PREFIX", dir.path());
        let result = prefix();
        std::env::remove_var("D_PREFIX");
        assert_eq!(result, dir.path());
    }

    #[test]
    fn bin_dir_is_under_prefix() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("D_PREFIX", dir.path());
        let b = bin_dir();
        std::env::remove_var("D_PREFIX");
        assert_eq!(b, dir.path().join("bin"));
    }

    #[test]
    fn active_version_returns_none_when_missing() {
        with_temp_prefix(|_| {
            assert_eq!(active_version(), None);
        });
    }

    #[test]
    fn active_version_reads_marker_file() {
        with_temp_prefix(|base| {
            fs::write(base.join(".active"), "v1.40.0").unwrap();
            assert_eq!(active_version(), Some("v1.40.0".to_string()));
        });
    }

    #[test]
    fn active_version_trims_whitespace() {
        with_temp_prefix(|base| {
            fs::write(base.join(".active"), "v1.40.0\n").unwrap();
            assert_eq!(active_version(), Some("v1.40.0".to_string()));
        });
    }

    #[cfg(unix)]
    #[test]
    fn activate_creates_symlink_and_marker() {
        with_temp_prefix(|base| {
            std::env::set_var("D_CACHE_DIR", base.join("versions"));
            let vdir = base.join("versions").join("v1.40.0");
            fs::create_dir_all(&vdir).unwrap();
            fs::write(vdir.join("deno"), b"#!/bin/sh\necho hi").unwrap();

            activate("v1.40.0").unwrap();

            let link = base.join("bin").join("deno");
            assert!(link.symlink_metadata().is_ok(), "symlink should exist");
            assert_eq!(active_version(), Some("v1.40.0".to_string()));
            std::env::remove_var("D_CACHE_DIR");
        });
    }

    #[cfg(unix)]
    #[test]
    fn activate_replaces_existing_symlink() {
        with_temp_prefix(|base| {
            std::env::set_var("D_CACHE_DIR", base.join("versions"));
            for v in &["v1.40.0", "v2.0.0"] {
                let vdir = base.join("versions").join(v);
                fs::create_dir_all(&vdir).unwrap();
                fs::write(vdir.join("deno"), b"#!/bin/sh").unwrap();
            }
            activate("v1.40.0").unwrap();
            activate("v2.0.0").unwrap();
            assert_eq!(active_version(), Some("v2.0.0".to_string()));
            std::env::remove_var("D_CACHE_DIR");
        });
    }

    #[cfg(unix)]
    #[test]
    fn uninstall_removes_symlink_and_marker() {
        with_temp_prefix(|base| {
            std::env::set_var("D_CACHE_DIR", base.join("versions"));
            let vdir = base.join("versions").join("v1.40.0");
            fs::create_dir_all(&vdir).unwrap();
            fs::write(vdir.join("deno"), b"#!/bin/sh").unwrap();
            activate("v1.40.0").unwrap();
            uninstall().unwrap();
            let link = base.join("bin").join("deno");
            assert!(!link.exists() && link.symlink_metadata().is_err());
            assert!(active_version().is_none());
            std::env::remove_var("D_CACHE_DIR");
        });
    }

    #[test]
    fn uninstall_ok_when_nothing_installed() {
        with_temp_prefix(|_| {
            assert!(uninstall().is_ok());
        });
    }
}
