use anyhow::{Context, Result};
use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::process::Command;

use crate::{arch, cache, releases, symlink};

/// Install a Deno version and activate it.
pub fn install(version_str: &str) -> Result<()> {
    let tag = releases::resolve_tag(version_str)?;

    if cache::is_cached(&tag) {
        println!("Version {tag} is already cached, activating...");
    } else {
        println!("Downloading Deno {tag}...");
        let url = arch::download_url(&tag);
        download_version(&url, &tag)?;
    }

    println!("Activating Deno {tag}...");
    symlink::activate(&tag)?;
    println!("Installed Deno {tag} successfully.");
    Ok(())
}

/// Download a version into cache without activating it.
pub fn download_only(version_str: &str) -> Result<()> {
    let tag = releases::resolve_tag(version_str)?;
    if cache::is_cached(&tag) {
        println!("Version {tag} is already cached.");
        return Ok(());
    }
    println!("Downloading Deno {tag}...");
    let url = arch::download_url(&tag);
    download_version(&url, &tag)
}

/// Run a cached Deno version with given arguments.
pub fn run(version_str: &str, args: &[String]) -> Result<()> {
    let tag = releases::resolve_tag(version_str)?;

    if !cache::is_cached(&tag) {
        println!("Version {tag} is not cached. Downloading...");
        let url = arch::download_url(&tag);
        download_version(&url, &tag)?;
    }

    let binary = cache::deno_binary(&tag);
    let status = Command::new(&binary)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run deno {tag}"))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn download_version(url: &str, tag: &str) -> Result<()> {
    let dest_dir = cache::version_dir(tag);
    fs::create_dir_all(&dest_dir).context("Failed to create cache directory")?;

    let tmp_path = dest_dir.with_extension("zip");

    {
        let client = reqwest::blocking::Client::new();
        let mut resp = client
            .get(url)
            .header("User-Agent", "d-deno-version-manager")
            .send()
            .context("HTTP request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            fs::remove_dir_all(&dest_dir).ok();
            anyhow::bail!("Download failed: server returned HTTP {status} for {url}");
        }

        let total = resp.content_length().unwrap_or(0);
        let file = fs::File::create(&tmp_path).context("Failed to create temp file")?;
        let mut writer = BufWriter::new(file);

        let mut downloaded = 0u64;
        let mut buf = vec![0u8; 65536];
        loop {
            let n = resp.read(&mut buf)?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n])?;
            downloaded += n as u64;
            if total > 0 {
                if let Some(pct) = downloaded.saturating_mul(100).checked_div(total) {
                    print!("\r  {downloaded}/{total} bytes ({pct}%)");
                    io::stdout().flush()?;
                }
            }
        }
        println!();
    }

    extract_zip(&tmp_path, &dest_dir)?;
    fs::remove_file(&tmp_path).ok();

    // Deno zips contain a single `deno` binary at the root — no flattening needed.
    // Make it executable on Unix.
    #[cfg(unix)]
    {
        let binary = cache::deno_binary(tag);
        if binary.exists() {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary, perms)?;
        }
    }

    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(archive).context("Failed to open zip")?;
    let mut zip = zip::ZipArchive::new(file).context("Failed to read zip")?;
    zip.extract(dest).context("Failed to extract zip")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_dirs<F: FnOnce(&std::path::Path, &std::path::Path)>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let cache = tempfile::tempdir().expect("tempdir");
        let prefix = tempfile::tempdir().expect("tempdir");
        std::env::set_var("D_CACHE_DIR", cache.path());
        std::env::set_var("D_PREFIX", prefix.path());
        f(cache.path(), prefix.path());
        std::env::remove_var("D_CACHE_DIR");
        std::env::remove_var("D_PREFIX");
    }

    /// Build a minimal valid ZIP archive in memory containing a single file.
    fn make_zip_bytes(filename: &str, content: &[u8]) -> Vec<u8> {
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<'_, zip::write::ExtendedFileOptions> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file(filename, options).unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn extract_zip_extracts_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_bytes = make_zip_bytes("deno", b"#!/bin/sh");
        let zip_path = tmp.path().join("deno.zip");
        fs::write(&zip_path, &zip_bytes).unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();
        extract_zip(&zip_path, &dest).unwrap();

        assert!(dest.join("deno").exists());
    }

    #[test]
    fn download_only_skips_if_already_cached() {
        with_temp_dirs(|cache, _prefix| {
            let vdir = cache.join("v1.40.0");
            fs::create_dir_all(&vdir).unwrap();
            fs::write(vdir.join("deno"), b"fake").unwrap();
            // This resolves the exact 3-part tag without network
            let result = download_only("1.40.0");
            assert!(result.is_ok(), "should skip download when cached: {result:?}");
        });
    }
}
