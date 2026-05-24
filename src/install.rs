use anyhow::{Context, Result};
use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::process::Command;

use console::style;

use crate::{arch, cache, releases, symlink};

/// Returns `true` if the input is a symbolic alias resolved entirely via network.
fn is_alias(s: &str) -> bool {
    matches!(
        s,
        "lts" | "stable" | "current" | "latest" | "canary" | "nightly" | "next" | "edge" | "beta"
    )
}

/// Returns `true` if the input looks like a bare version number.
fn looks_like_version(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'x' | 'X'))
}

/// Returns `true` if the input looks like a git commit SHA (7-40 hex chars
/// with at least one letter `a`-`f`).
fn is_sha_input(s: &str) -> bool {
    let n = s.len();
    (7..=40).contains(&n)
        && s.chars().all(|c| c.is_ascii_hexdigit())
        && s.chars().any(|c| matches!(c, 'a'..='f' | 'A'..='F'))
}

/// Extract the base version (before any `+sha`) and the optional SHA from a
/// resolved tag. Channel suffixes like `-dev.N` are stripped at the first `-`.
fn extract_ver_sha(tag: &str) -> (String, Option<&str>) {
    let (ver_part, sha) = tag
        .split_once('+')
        .map_or((tag, None), |(v, s)| (v, Some(s)));
    let clean_ver = ver_part.split('-').next().unwrap_or(ver_part).to_string();
    (clean_ver, sha)
}

/// Query the installed deno binary to determine the canonical cache key.
///
/// Stable: `deno 2.3.6 (stable, ...)` → `("2.3.6", None)`\
/// Canary: `deno 2.3.6+abc1234d (canary, ...)` → `("2.3.6", Some("abc1234d"))` (7 chars)
fn query_binary_version(binary_path: &Path) -> Result<(String, Option<String>)> {
    let out = Command::new(binary_path)
        .arg("--version")
        .output()
        .context("Failed to run deno --version")?;
    let raw = String::from_utf8_lossy(&out.stdout);
    let first_line = raw.lines().next().unwrap_or("").trim();
    // Strip "deno " prefix
    let rest = first_line.strip_prefix("deno ").unwrap_or(first_line);
    // rest = "2.3.6 (stable...)" or "2.3.6+abc1234d (canary...)"
    let ver_part = rest.split_whitespace().next().unwrap_or("");
    if let Some((ver, sha)) = ver_part.split_once('+') {
        let short_sha = sha[..sha.len().min(7)].to_string();
        Ok((ver.to_string(), Some(short_sha)))
    } else {
        Ok((ver_part.to_string(), None))
    }
}

/// Activate an already-cached version (update the symlink).
fn activate_cached(tag: &str) -> Result<()> {
    if symlink::active_version().as_deref() == Some(tag) {
        println!(
            "{} Deno {} is already the active version.",
            style("\u{2713}").green().bold(),
            style(tag).cyan().bold(),
        );
        return Ok(());
    }
    let from = symlink::active_version();
    match &from {
        Some(f) => println!(
            "{} Activating Deno {} \u{2192} {}...",
            style("\u{25c6}").magenta(),
            style(f).cyan().bold(),
            style(tag).cyan().bold(),
        ),
        None => println!(
            "{} Activating Deno {}...",
            style("\u{25c6}").magenta(),
            style(tag).cyan().bold(),
        ),
    }
    symlink::activate(tag)?;
    println!(
        "{} Installed Deno {} successfully.",
        style("\u{2713}").green().bold(),
        style(tag).cyan().bold(),
    );
    Ok(())
}

/// Install a Deno version and activate it.
pub fn install(version_str: &str) -> Result<()> {
    let v = version_str.trim();

    // 1. Pre-resolve cache check — skip network for version/SHA inputs
    if !is_alias(v) && !v.contains('+') {
        if is_sha_input(v) {
            if let Some(cached) = cache::find_by_sha(v) {
                return activate_cached(&cached);
            }
        } else if looks_like_version(v) {
            let prefix = v.trim_end_matches(".x").trim_end_matches(".X");
            if let Some(cached) = cache::find_by_version_prefix(prefix) {
                return activate_cached(&cached);
            }
        }
    }

    // 2. Resolve via network
    let tag = releases::resolve_tag(v)?;

    // 3. Post-resolve cache check
    {
        let (ver_prefix, tag_sha) = extract_ver_sha(&tag);
        if let Some(cached) = cache::find_by_version_prefix(&ver_prefix) {
            let sha_ok = match (tag_sha, cache::cache_key_sha(&cached)) {
                (Some(ts), Some(cs)) => cache::sha_matches(cs, ts),
                (None, _) => true,
                (Some(_), None) => false,
            };
            if sha_ok {
                return activate_cached(&cached);
            }
        }
    }

    // 4. Download if not already cached
    if !cache::is_cached(&tag) {
        println!(
            "{} Downloading Deno {}...",
            style("\u{2b07}").cyan(),
            style(&tag).cyan().bold(),
        );
        let url = arch::download_url(&tag);
        download_version(&url, &tag)?;
    }

    // 5. Query the installed binary to get the canonical cache key
    let binary = cache::deno_binary(&tag);
    let canonical = match query_binary_version(&binary) {
        Ok((ver, sha_opt)) => sha_opt.map_or_else(|| ver.clone(), |s| format!("{ver}+{s}")),
        Err(_) => tag.clone(),
    };
    if canonical != tag {
        cache::rename_version(&tag, &canonical)?;
    }

    activate_cached(&canonical)
}

/// Download a version into cache without activating it.
pub fn download_only(version_str: &str) -> Result<()> {
    let v = version_str.trim();

    // Pre-resolve cache check
    if !is_alias(v) && !v.contains('+') {
        if is_sha_input(v) {
            if let Some(cached) = cache::find_by_sha(v) {
                println!("Version {cached} is already cached.");
                return Ok(());
            }
        } else if looks_like_version(v) {
            let prefix = v.trim_end_matches(".x").trim_end_matches(".X");
            if let Some(cached) = cache::find_by_version_prefix(prefix) {
                println!("Version {cached} is already cached.");
                return Ok(());
            }
        }
    }

    let tag = releases::resolve_tag(v)?;

    // Post-resolve cache check
    {
        let (ver_prefix, tag_sha) = extract_ver_sha(&tag);
        if let Some(cached) = cache::find_by_version_prefix(&ver_prefix) {
            let sha_ok = match (tag_sha, cache::cache_key_sha(&cached)) {
                (Some(ts), Some(cs)) => cache::sha_matches(cs, ts),
                (None, _) => true,
                (Some(_), None) => false,
            };
            if sha_ok {
                println!("Version {cached} is already cached.");
                return Ok(());
            }
        }
    }

    if cache::is_cached(&tag) {
        println!("Version {tag} is already cached.");
        return Ok(());
    }
    println!("Downloading Deno {tag}...");
    let url = arch::download_url(&tag);
    download_version(&url, &tag)?;
    let binary = cache::deno_binary(&tag);
    if let Ok((ver, sha_opt)) = query_binary_version(&binary) {
        let canonical = sha_opt.map_or_else(|| ver.clone(), |s| format!("{ver}+{s}"));
        if canonical != tag {
            cache::rename_version(&tag, &canonical)?;
        }
    }
    Ok(())
}

/// Run a cached Deno version with given arguments.
pub fn run(version_str: &str, args: &[String]) -> Result<()> {
    let v = version_str.trim();

    // Pre-resolve cache check
    if !is_alias(v) && !v.contains('+') {
        if is_sha_input(v) {
            if let Some(cached) = cache::find_by_sha(v) {
                return run_cached(&cached, args);
            }
        } else if looks_like_version(v) {
            let prefix = v.trim_end_matches(".x").trim_end_matches(".X");
            if let Some(cached) = cache::find_by_version_prefix(prefix) {
                return run_cached(&cached, args);
            }
        }
    }

    let tag = releases::resolve_tag(v)?;

    // Post-resolve cache check
    let resolved_tag = {
        let (ver_prefix, tag_sha) = extract_ver_sha(&tag);
        if let Some(cached) = cache::find_by_version_prefix(&ver_prefix) {
            let sha_ok = match (tag_sha, cache::cache_key_sha(&cached)) {
                (Some(ts), Some(cs)) => cache::sha_matches(cs, ts),
                (None, _) => true,
                (Some(_), None) => false,
            };
            if sha_ok {
                return run_cached(&cached, args);
            }
        }
        tag
    };

    if !cache::is_cached(&resolved_tag) {
        println!("Version {resolved_tag} is not cached. Downloading...");
        let url = arch::download_url(&resolved_tag);
        download_version(&url, &resolved_tag)?;
    }

    let binary = cache::deno_binary(&resolved_tag);
    let canonical = match query_binary_version(&binary) {
        Ok((ver, sha_opt)) => sha_opt.map_or_else(|| ver.clone(), |s| format!("{ver}+{s}")),
        Err(_) => resolved_tag.clone(),
    };
    if canonical != resolved_tag {
        cache::rename_version(&resolved_tag, &canonical)?;
    }
    run_cached(&canonical, args)
}

fn run_cached(tag: &str, args: &[String]) -> Result<()> {
    let binary = cache::deno_binary(tag);
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

/// Remove a cached version, or prompt for interactive selection if no version is given.
pub fn remove_version(version: Option<String>) -> Result<()> {
    if let Some(v) = version {
        if cache::is_cached(&v) {
            cache::remove(&v)?;
        } else if is_sha_input(&v) {
            if let Some(matched) = cache::find_by_sha(&v) {
                cache::remove(&matched)?;
            } else {
                println!("Version '{v}' is not cached.");
            }
        } else if let Some(matched) = cache::find_by_version_prefix(&v) {
            cache::remove(&matched)?;
        } else {
            println!("Version '{v}' is not cached.");
        }
        return Ok(());
    }
    let versions = cache::cached_versions()?;
    if versions.is_empty() {
        println!("No cached versions to remove.");
        return Ok(());
    }
    let active = symlink::active_version();
    let items: Vec<String> = versions
        .iter()
        .map(|v| {
            if Some(v.as_str()) == active.as_deref() {
                format!("{v}  (active)")
            } else {
                v.clone()
            }
        })
        .collect();
    let idx = dialoguer::Select::new()
        .with_prompt("Select a version to remove")
        .items(&items)
        .interact()?;
    cache::remove(&versions[idx])?;
    Ok(())
}

const GITHUB_REPO: &str = "THernandez03/d";

fn self_artifact() -> String {
    let name = env!("CARGO_PKG_NAME");
    let os_arch = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x64"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "linux-arm64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "darwin-x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "darwin-arm64"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x64"
    } else {
        "linux-x64"
    };
    if cfg!(target_os = "windows") {
        format!("{name}-{os_arch}.exe")
    } else {
        format!("{name}-{os_arch}")
    }
}

/// Self-update this version manager binary to the latest GitHub release.
pub fn update_self() -> Result<()> {
    let name = env!("CARGO_PKG_NAME");
    println!("{} Checking for {} updates...", style("◆").cyan(), name);
    let client = reqwest::blocking::Client::new();
    let release: serde_json::Value = client
        .get(format!(
            "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
        ))
        .header("User-Agent", format!("{name}-version-manager"))
        .send()
        .context("Failed to fetch latest release info")?
        .json()
        .context("Failed to parse release JSON")?;
    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in GitHub release response"))?;
    let current = env!("CARGO_PKG_VERSION");
    let remote = tag.trim_start_matches('v');
    if remote == current {
        println!(
            "{} {} is already up to date ({})",
            style("✓").green().bold(),
            name,
            style(current).cyan().bold()
        );
        return Ok(());
    }
    println!(
        "{} Updating {} {} \u{2192} {}...",
        style("⬇").cyan(),
        name,
        style(current).dim(),
        style(remote).cyan().bold()
    );
    let artifact = self_artifact();
    let url = format!("https://github.com/{GITHUB_REPO}/releases/download/{tag}/{artifact}");
    let exe = std::env::current_exe().context("Failed to locate current executable")?;
    let tmp = exe.with_extension("update-tmp");
    {
        let mut resp = client
            .get(&url)
            .header("User-Agent", format!("{name}-version-manager"))
            .send()
            .context("Failed to download update")?;
        if !resp.status().is_success() {
            anyhow::bail!("Download failed: HTTP {} for {}", resp.status(), url);
        }
        let file = fs::File::create(&tmp).context("Failed to create temp file for update")?;
        let mut writer = BufWriter::new(file);
        let mut buf = vec![0u8; 65536];
        loop {
            let n = resp.read(&mut buf).context("Read error during download")?;
            if n == 0 {
                break;
            }
            writer
                .write_all(&buf[..n])
                .context("Write error during download")?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permission")?;
    }
    fs::rename(&tmp, &exe).context("Failed to replace current binary")?;
    println!(
        "{} {} updated to {}.",
        style("✓").green().bold(),
        name,
        style(remote).cyan().bold()
    );
    Ok(())
}

/// Uninstall this version manager completely (removes cache, prefix directory, and the binary).
pub fn uninstall_self(yes: bool) -> Result<()> {
    let name = env!("CARGO_PKG_NAME");
    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "This will remove all cached versions and the {name} binary. Continue?"
            ))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("{}", style("Aborted.").yellow());
            return Ok(());
        }
    }
    println!("Uninstalling {}...", style(name).cyan().bold());
    let prefix = symlink::prefix();
    if prefix.exists() {
        fs::remove_dir_all(&prefix)
            .with_context(|| format!("Failed to remove {}", prefix.display()))?;
        println!("  {} Removed {}", style("✓").green(), prefix.display());
    }
    let exe = std::env::current_exe().context("Failed to locate current executable")?;
    fs::remove_file(&exe).with_context(|| format!("Failed to remove {}", exe.display()))?;
    println!("  {} Removed {}", style("✓").green(), exe.display());
    println!();
    println!(
        "{} {} uninstalled. Remove {} from your PATH if needed.",
        style("✓").green().bold(),
        name,
        exe.parent()
            .map_or_else(String::new, |p| p.display().to_string())
    );
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
            assert!(
                result.is_ok(),
                "should skip download when cached: {result:?}"
            );
        });
    }

    // ── is_alias ───────────────────────────────────────────────────

    #[test]
    fn is_alias_known_aliases() {
        assert!(is_alias("lts"));
        assert!(is_alias("stable"));
        assert!(is_alias("current"));
        assert!(is_alias("latest"));
        assert!(is_alias("canary"));
        assert!(is_alias("nightly"));
        assert!(is_alias("next"));
        assert!(is_alias("edge"));
        assert!(is_alias("beta"));
    }

    #[test]
    fn is_alias_version_not_alias() {
        assert!(!is_alias("2.3.6"));
        assert!(!is_alias("abc1234d"));
        assert!(!is_alias(""));
    }

    // ── looks_like_version ──────────────────────────────────────────

    #[test]
    fn looks_like_version_semver() {
        assert!(looks_like_version("2.3.6"));
        assert!(looks_like_version("1.40.0"));
    }

    #[test]
    fn looks_like_version_x_notation() {
        assert!(looks_like_version("2.x"));
        assert!(looks_like_version("2.3.X"));
    }

    #[test]
    fn looks_like_version_non_versions() {
        assert!(!looks_like_version("canary"));
        assert!(!looks_like_version("v1.2.3"));
        assert!(!looks_like_version("abc1234d"));
    }

    // ── is_sha_input ───────────────────────────────────────────────

    #[test]
    fn is_sha_input_valid() {
        assert!(is_sha_input("abc1234d"));
        assert!(is_sha_input("abc1234def5678"));
    }

    #[test]
    fn is_sha_input_too_short() {
        assert!(!is_sha_input("abc123"));
    }

    #[test]
    fn is_sha_input_all_digits_rejected() {
        assert!(!is_sha_input("12345678"));
    }

    #[test]
    fn is_sha_input_non_hex_rejected() {
        assert!(!is_sha_input("abc1234g"));
    }

    // ── extract_ver_sha ─────────────────────────────────────────────

    #[test]
    fn extract_ver_sha_with_sha() {
        let (ver, sha) = extract_ver_sha("2.3.6+abc1234def");
        assert_eq!(ver, "2.3.6");
        assert_eq!(sha, Some("abc1234def"));
    }

    #[test]
    fn extract_ver_sha_without_sha() {
        let (ver, sha) = extract_ver_sha("2.3.6");
        assert_eq!(ver, "2.3.6");
        assert!(sha.is_none());
    }

    #[test]
    fn extract_ver_sha_strips_channel_suffix() {
        let (ver, sha) = extract_ver_sha("2.3.6-canary.abc1234d+abc1234de");
        assert_eq!(ver, "2.3.6");
        assert_eq!(sha, Some("abc1234de"));
    }

    #[test]
    fn extract_ver_sha_channel_no_sha() {
        let (ver, sha) = extract_ver_sha("2.3.6-canary.abc1234d");
        assert_eq!(ver, "2.3.6");
        assert!(sha.is_none());
    }
}
