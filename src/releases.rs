use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

const RELEASES_URL: &str = "https://api.github.com/repos/denoland/deno/releases";

#[derive(Debug, Deserialize, Clone)]
pub struct GhRelease {
    pub tag_name: String,
    pub prerelease: bool,
}

/// Fetch the recent release list from GitHub.
pub fn fetch_releases(per_page: u32) -> Result<Vec<GhRelease>> {
    let client = Client::new();
    let url = format!("{RELEASES_URL}?per_page={per_page}");
    let releases: Vec<GhRelease> = client
        .get(&url)
        .header("User-Agent", "d-deno-version-manager")
        .send()
        .context("Failed to fetch Deno releases from GitHub")?
        .json()
        .context("Failed to parse GitHub releases JSON")?;
    Ok(releases)
}

/// Print recent Deno releases (latest 20).
pub fn list_remote() -> Result<()> {
    let releases = fetch_releases(20)?;
    println!("Available Deno versions (recent 20):");
    for r in &releases {
        let pre = if r.prerelease { " (pre-release)" } else { "" };
        println!("  {}{}", r.tag_name, pre);
    }
    Ok(())
}

/// Resolve a user-supplied version string to an exact GitHub release tag.
///
/// Aliases:
/// - `"latest"` / `"stable"` / `"lts"` / `""` → latest stable release
/// - `"canary"` / `"next"` → `canary` tag (Deno's nightly)
/// - `"1.40"` / `"v1.40"` → latest stable in that minor line
/// - `"1.40.0"` / `"v1.40.0"` → exact tag (no network needed)
pub fn resolve_tag(version_str: &str) -> Result<String> {
    let v = version_str.trim();

    // canary short-circuits — no network needed
    if v == "canary" || v == "next" {
        return Ok("canary".to_string());
    }

    // Strip leading `v` so bare semver and `v`-prefixed both work
    let bare = v.strip_prefix('v').unwrap_or(v);

    // Three-part exact version — skip network
    if bare.split('.').count() >= 3 && bare.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Ok(format!("v{bare}"));
    }

    // Need the release list for aliases and prefix matching
    let releases = fetch_releases(100)?;
    resolve_from(v, &releases)
}

/// Pure resolver operating on a pre-fetched slice (used by tests).
pub fn resolve_from(version_str: &str, releases: &[GhRelease]) -> Result<String> {
    let v = version_str.trim();

    // Canary — project-native nightly
    if v == "canary" || v == "next" {
        return Ok("canary".to_string());
    }

    let bare = v.strip_prefix('v').unwrap_or(v);

    // Latest stable / lts aliases
    if bare.is_empty() || bare == "latest" || bare == "stable" || bare == "lts" {
        return releases
            .iter()
            .find(|r| !r.prerelease)
            .map(|r| r.tag_name.clone())
            .ok_or_else(|| anyhow::anyhow!("No stable Deno release found on GitHub"));
    }

    // Strip trailing `.x` / `.X`
    let prefix = bare.trim_end_matches(".x").trim_end_matches(".X");

    // Prefix match: e.g. "1.40" matches "v1.40.3"
    let needle = format!("v{prefix}.");
    releases
        .iter()
        .find(|r| {
            !r.prerelease
                && (r.tag_name.starts_with(&needle) || r.tag_name == format!("v{prefix}"))
        })
        .map(|r| r.tag_name.clone())
        .ok_or_else(|| anyhow::anyhow!("No stable Deno release found matching '{version_str}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_release(tag: &str, prerelease: bool) -> GhRelease {
        GhRelease {
            tag_name: tag.to_string(),
            prerelease,
        }
    }

    fn stable_releases() -> Vec<GhRelease> {
        vec![
            make_release("v2.0.0", false),
            make_release("v1.46.3", false),
            make_release("v1.46.2", false),
            make_release("v1.40.0", false),
            make_release("v1.40.0-rc.1", true),
        ]
    }

    #[test]
    fn resolve_canary_without_network() {
        assert_eq!(resolve_from("canary", &stable_releases()).unwrap(), "canary");
    }

    #[test]
    fn resolve_next_without_network() {
        assert_eq!(resolve_from("next", &stable_releases()).unwrap(), "canary");
    }

    #[test]
    fn resolve_latest_returns_first_stable() {
        assert_eq!(resolve_from("latest", &stable_releases()).unwrap(), "v2.0.0");
    }

    #[test]
    fn resolve_stable_returns_first_stable() {
        assert_eq!(resolve_from("stable", &stable_releases()).unwrap(), "v2.0.0");
    }

    #[test]
    fn resolve_lts_returns_first_stable() {
        assert_eq!(resolve_from("lts", &stable_releases()).unwrap(), "v2.0.0");
    }

    #[test]
    fn resolve_prefix_minor() {
        assert_eq!(resolve_from("1.46", &stable_releases()).unwrap(), "v1.46.3");
    }

    #[test]
    fn resolve_prefix_major() {
        assert_eq!(resolve_from("1", &stable_releases()).unwrap(), "v1.46.3");
    }

    #[test]
    fn resolve_prefix_with_x_notation() {
        assert_eq!(resolve_from("1.46.x", &stable_releases()).unwrap(), "v1.46.3");
    }

    #[test]
    fn resolve_exact_version() {
        assert_eq!(resolve_from("v1.40.0", &stable_releases()).unwrap(), "v1.40.0");
    }

    #[test]
    fn resolve_skips_prerelease() {
        // v1.40.0-rc.1 is prerelease; "1.40" should resolve to v1.40.0
        assert_eq!(resolve_from("1.40", &stable_releases()).unwrap(), "v1.40.0");
    }

    #[test]
    fn resolve_errors_on_unknown() {
        assert!(resolve_from("99", &stable_releases()).is_err());
    }

    #[test]
    fn resolve_errors_on_empty_list() {
        assert!(resolve_from("latest", &[]).is_err());
    }
}
