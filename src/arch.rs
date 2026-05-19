/// Returns the Deno target triple for the current platform.
/// Deno release filenames use the pattern: `deno-{target}.zip`
/// e.g. `deno-x86_64-unknown-linux-gnu.zip`
#[must_use]
pub const fn target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "x86_64-unknown-linux-gnu";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "aarch64-unknown-linux-gnu";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "x86_64-pc-windows-msvc";
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "aarch64-pc-windows-msvc";
    #[cfg(not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(
            target_os = "macos",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(
            target_os = "windows",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
    )))]
    return "x86_64-unknown-linux-gnu"; // fallback
}

/// Build the download URL for a specific Deno release tag.
///
/// Uses dl.deno.land:
/// - Release: `https://dl.deno.land/release/{tag}/deno-{target}.zip`
/// - Canary:  `https://dl.deno.land/canary/{sha}/deno-{target}.zip`
///
/// Cache keys for canary are `"canary-{sha}"`; the SHA is extracted for the URL.
#[must_use]
pub fn download_url(tag: &str) -> String {
    let tgt = target();
    tag.strip_prefix("canary-").map_or_else(
        || format!("https://dl.deno.land/release/{tag}/deno-{tgt}.zip"),
        |sha| format!("https://dl.deno.land/canary/{sha}/deno-{tgt}.zip"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_non_empty() {
        assert!(!target().is_empty());
    }

    #[test]
    fn target_contains_dash() {
        assert!(target().contains('-'));
    }

    #[test]
    fn target_is_known_triple() {
        let known = [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
        ];
        assert!(known.contains(&target()), "unexpected target: {}", target());
    }

    #[test]
    fn download_url_contains_tag() {
        let url = download_url("v1.40.0");
        assert!(url.contains("v1.40.0"), "url: {url}");
    }

    #[test]
    fn download_url_starts_with_dl_deno_land() {
        let url = download_url("v1.40.0");
        assert!(
            url.starts_with("https://dl.deno.land/release/"),
            "url: {url}"
        );
    }

    #[test]
    fn download_url_ends_with_zip() {
        let url = download_url("v1.40.0");
        assert!(
            std::path::Path::new(&url)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip")),
            "url: {url}"
        );
    }

    #[test]
    fn download_url_contains_target() {
        let url = download_url("v1.40.0");
        assert!(url.contains(target()), "url: {url}");
    }

    #[test]
    fn download_url_canary_with_sha() {
        let sha = "abc123def456";
        let url = download_url(&format!("canary-{sha}"));
        assert!(
            url.starts_with("https://dl.deno.land/canary/"),
            "url: {url}"
        );
        assert!(url.contains(sha), "url: {url}");
        assert!(
            std::path::Path::new(&url)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip")),
            "url: {url}"
        );
    }
}
