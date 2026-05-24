use anyhow::{Context, Result};
use reqwest::blocking::{Client, Response};
use serde::Deserialize;

const RELEASES_URL: &str = "https://api.github.com/repos/denoland/deno/releases";

#[derive(Debug, Deserialize, Clone)]
pub struct GhRelease {
    pub tag_name: String,
    pub prerelease: bool,
}

/// Return the response unchanged if the status is 2xx, or bail with the
/// HTTP status, request URL, and the pretty-printed GitHub error JSON body.
fn check_github_response(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let url = response.url().to_string();
    let body = response.text().unwrap_or_default();
    let pretty = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or(body);
    anyhow::bail!("GitHub API error ({status}) for {url}\n\n{pretty}")
}

/// Fetch the current canary commit SHA from `dl.deno.land/canary-latest.txt`,
/// then read the version from the Deno Cargo.toml at that exact commit.
/// Returns a cache key like `"2.8.0+{full_sha}"` where the SHA is 40 chars —
/// the full SHA is required by `dl.deno.land/canary/{sha}/` download URLs.
fn resolve_canary_tag() -> Result<String> {
    let client = Client::new();
    let sha = client
        .get("https://dl.deno.land/canary-latest.txt")
        .header("User-Agent", "d-deno-version-manager")
        .send()
        .context("Failed to fetch Deno canary SHA")?
        .text()
        .context("Failed to read Deno canary SHA response")?;
    let sha = sha.trim().to_string();
    anyhow::ensure!(!sha.is_empty(), "Deno canary SHA was empty");

    // Read the version from Cargo.toml at this exact SHA.
    // Deno has no GitHub pre-releases for canary, so we query the source directly.
    let version = fetch_version_at_sha(&client, &sha).unwrap_or_else(|_| "canary".to_string());

    // Store the FULL SHA — arch.rs extracts it for the dl.deno.land download URL
    // which requires the exact 40-char SHA. list.rs truncates to 9 chars for display.
    Ok(format!("{version}+{sha}"))
}

/// Fetch the Deno version string from `cli/Cargo.toml` at the given git SHA.
fn fetch_version_at_sha(client: &Client, sha: &str) -> Result<String> {
    let url = format!("https://raw.githubusercontent.com/denoland/deno/{sha}/cli/Cargo.toml");
    let body = client
        .get(&url)
        .header("User-Agent", "d-deno-version-manager")
        .send()
        .context("Failed to fetch Deno cli/Cargo.toml")?
        .text()
        .context("Failed to read Deno cli/Cargo.toml")?;
    // Find the first `version = "x.y.z"` line where x.y.z is a numeric semver.
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version = \"") {
            if let Some(ver) = rest.strip_suffix('"') {
                if ver.split('.').count() == 3
                    && ver.chars().all(|c| c.is_ascii_digit() || c == '.')
                {
                    return Ok(ver.to_string());
                }
            }
        }
    }
    anyhow::bail!("Version not found in cli/Cargo.toml at SHA {sha}")
}

/// Fetch the recent release list from GitHub.
pub fn fetch_releases(per_page: u32) -> Result<Vec<GhRelease>> {
    let client = Client::new();
    let url = format!("{RELEASES_URL}?per_page={per_page}");
    let response = client
        .get(&url)
        .header("User-Agent", "d-deno-version-manager")
        .send()
        .context("Failed to fetch Deno releases from GitHub")?;
    let releases: Vec<GhRelease> = check_github_response(response)?
        .json()
        .context("Failed to parse Deno releases JSON")?;
    Ok(releases)
}

/// Fetch the latest stable Deno version using the `/releases/latest` endpoint.
/// Avoids the paginated array endpoint, which is less reliable when rate-limited.
fn resolve_latest_stable_tag() -> Result<String> {
    let client = Client::new();
    let url = format!("{RELEASES_URL}/latest");
    let response = client
        .get(&url)
        .header("User-Agent", "d-deno-version-manager")
        .send()
        .context("Failed to fetch latest Deno release")?;
    let release: serde_json::Value = check_github_response(response)?
        .json()
        .context("Failed to parse latest Deno release JSON")?;
    release["tag_name"]
        .as_str()
        .map(|t| t.trim_start_matches('v').to_string())
        .ok_or_else(|| anyhow::anyhow!("No tag_name in latest Deno release response"))
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
/// - `"latest"` / `"stable"` / `"lts"` / `"current"` / `""` → latest stable release
/// - `"canary"` / `"next"` / `"nightly"` / `"edge"` → canary tag (Deno's nightly)
/// - `"1.40"` → latest stable in that minor line
/// - `"1.40.0"` → exact tag (no network needed)
pub fn resolve_tag(version_str: &str) -> Result<String> {
    let v = version_str.trim();

    // Already-resolved canary tag like "2.8.0+{sha}" — return as-is.
    if v.contains('+') {
        return Ok(v.to_string());
    }

    // Reject v-prefixed version strings
    if v.starts_with('v') && v[1..].starts_with(|c: char| c.is_ascii_digit()) {
        anyhow::bail!("No Deno release found matching '{v}'");
    }

    if v == "beta" {
        anyhow::bail!("'beta' channel is not supported for Deno");
    }

    // Canary short-circuits — fetches the current SHA from dl.deno.land
    if matches!(v, "canary" | "next" | "nightly" | "edge") {
        return resolve_canary_tag();
    }

    // Three-part exact version — skip network
    if v.split('.').count() >= 3 && v.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Ok(v.to_string());
    }

    // lts/stable/current/latest — use /releases/latest directly.
    if v.is_empty() || matches!(v, "stable" | "lts" | "current" | "latest") {
        return resolve_latest_stable_tag();
    }

    // Prefix match (e.g. "1.46") — needs the full release list.
    let releases = fetch_releases(100)?;
    resolve_from(v, &releases)
}

/// Pure resolver operating on a pre-fetched slice (used by tests).
pub fn resolve_from(version_str: &str, releases: &[GhRelease]) -> Result<String> {
    let v = version_str.trim();

    // Reject v-prefixed version strings
    if v.starts_with('v') && v[1..].starts_with(|c: char| c.is_ascii_digit()) {
        anyhow::bail!("No Deno release found matching '{v}'");
    }

    if v == "beta" {
        anyhow::bail!("'beta' channel is not supported for Deno");
    }

    // Canary — project-native nightly
    if matches!(v, "canary" | "next" | "latest" | "nightly" | "edge") {
        return Ok("canary".to_string());
    }

    // Latest stable / lts / current aliases
    if v.is_empty() || matches!(v, "stable" | "lts" | "current") {
        return releases
            .iter()
            .find(|r| !r.prerelease)
            .map(|r| r.tag_name.trim_start_matches('v').to_string())
            .ok_or_else(|| anyhow::anyhow!("No stable Deno release found on GitHub"));
    }

    // Strip trailing `.x` / `.X`
    let prefix = v.trim_end_matches(".x").trim_end_matches(".X");

    // Prefix match: e.g. "1.40" matches "v1.40.3"
    let needle = format!("v{prefix}.");
    releases
        .iter()
        .find(|r| {
            !r.prerelease && (r.tag_name.starts_with(&needle) || r.tag_name == format!("v{prefix}"))
        })
        .map(|r| r.tag_name.trim_start_matches('v').to_string())
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
        // canary resolution now hits the network; resolve_from still returns
        // the bare "canary" string when called directly (used by aliases in
        // non-canary paths). The actual canary-{sha} form comes from resolve_tag.
        assert_eq!(
            resolve_from("canary", &stable_releases()).unwrap(),
            "canary"
        );
    }

    #[test]
    fn resolve_next_without_network() {
        assert_eq!(resolve_from("next", &stable_releases()).unwrap(), "canary");
    }

    #[test]
    fn resolve_latest_returns_canary() {
        assert_eq!(
            resolve_from("latest", &stable_releases()).unwrap(),
            "canary"
        );
    }

    #[test]
    fn resolve_nightly_returns_canary() {
        assert_eq!(
            resolve_from("nightly", &stable_releases()).unwrap(),
            "canary"
        );
    }

    #[test]
    fn resolve_edge_returns_canary() {
        assert_eq!(resolve_from("edge", &stable_releases()).unwrap(), "canary");
    }

    #[test]
    fn resolve_stable_returns_first_stable() {
        assert_eq!(resolve_from("stable", &stable_releases()).unwrap(), "2.0.0");
    }

    #[test]
    fn resolve_lts_returns_first_stable() {
        assert_eq!(resolve_from("lts", &stable_releases()).unwrap(), "2.0.0");
    }

    #[test]
    fn resolve_prefix_minor() {
        assert_eq!(resolve_from("1.46", &stable_releases()).unwrap(), "1.46.3");
    }

    #[test]
    fn resolve_prefix_major() {
        assert_eq!(resolve_from("1", &stable_releases()).unwrap(), "1.46.3");
    }

    #[test]
    fn resolve_prefix_with_x_notation() {
        assert_eq!(
            resolve_from("1.46.x", &stable_releases()).unwrap(),
            "1.46.3"
        );
    }

    #[test]
    fn resolve_exact_version_v_prefix_rejected() {
        assert!(resolve_from("v1.40.0", &stable_releases()).is_err());
    }

    #[test]
    fn resolve_current_returns_stable() {
        assert_eq!(
            resolve_from("current", &stable_releases()).unwrap(),
            "2.0.0"
        );
    }

    #[test]
    fn resolve_beta_returns_error() {
        assert!(resolve_from("beta", &stable_releases()).is_err());
    }

    #[test]
    fn resolve_skips_prerelease() {
        // v1.40.0-rc.1 is prerelease; "1.40" should resolve to v1.40.0
        assert_eq!(resolve_from("1.40", &stable_releases()).unwrap(), "1.40.0");
    }

    #[test]
    fn resolve_errors_on_unknown() {
        assert!(resolve_from("99", &stable_releases()).is_err());
    }

    #[test]
    fn resolve_errors_on_empty_list() {
        // "latest" now resolves to canary without needing the list;
        // use a stable alias that does require releases.
        assert!(resolve_from("stable", &[]).is_err());
    }
}
