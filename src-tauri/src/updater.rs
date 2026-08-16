//! Checking GitHub for a newer Emerald, and fetching the installer.
//!
//! # Why this is not Tauri's updater plugin
//!
//! Tauri ships an updater that downloads and swaps the application in place,
//! with no installer and no clicks. It refuses to do that unless the artefact
//! carries a minisign signature made with a private key held in CI, and that is
//! not a formality to work around — it is the only thing standing between an
//! auto-installing browser and anyone who can answer a DNS query. This
//! repository has no signing key (see the release notes: the builds are
//! unsigned, and saying so is cheaper than pretending otherwise), so the silent
//! path is closed and closing it is correct.
//!
//! What is here instead: ask the public releases API what the newest tag is,
//! and if it is newer than this build, download that platform's installer and
//! hand it to the operating system to run. The user clicks through the
//! installer they would have clicked through anyway. It is one click short of
//! automatic, and the click is the part doing the security.
//!
//! If a signing key is ever added, `docs/architecture.md` §10 records what
//! would need to change to move to the real updater.
//!
//! # What leaves the machine
//!
//! One HTTPS GET to a public, unauthenticated endpoint, carrying a User-Agent
//! and nothing else. No identifier, no account, no installation id, no counter.
//! It happens once when a window opens, never on a timer, and only while
//! `privacy.check_for_updates` is on.

use serde::Serialize;
use std::path::PathBuf;

/// The release *list*, not `/releases/latest`.
///
/// `/releases/latest` looks like the obvious endpoint and is the wrong one: it
/// excludes prereleases, and every Emerald release so far is marked as one. It
/// returned a flat 404 for this repository, so the update check could never
/// find anything no matter what was published — a bug that hid perfectly,
/// because a failed check is deliberately silent and "you are up to date" and
/// "the request 404'd" look identical from the outside.
///
/// The list endpoint includes prereleases. Drafts are invisible to it without
/// authentication, which is the behaviour we want anyway: a draft is not
/// released.
const RELEASES: &str =
    "https://api.github.com/repos/ZDStudios/Emerald/releases?per_page=20";

/// GitHub asks for a User-Agent and returns 403 without one.
const AGENT: &str = "Emerald-Browser";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// The released version, without a leading `v`.
    pub version: String,
    /// This build's version, for the "you are on X, Y is out" sentence.
    pub current: String,
    /// The release page, so there is always a way through by hand.
    pub url: String,
    /// Release notes, trimmed — enough to say what changed, not a wall.
    pub notes: String,
    /// Direct link to this platform's installer, when the release has one.
    pub asset_url: Option<String>,
    /// The installer's filename, so the UI can name what it will download.
    pub asset_name: Option<String>,
}

/// Compare two dotted version strings numerically.
///
/// String comparison gets this wrong the moment a version reaches double
/// digits — `"0.10.0" < "0.9.0"` lexically, which would strand everyone on
/// 0.9 forever, and would do it silently.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            // Drop any pre-release or build suffix before comparing numbers.
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(candidate), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// Does this asset filename look like the installer for the running platform?
///
/// Deliberately narrow. Picking the wrong file here means downloading an
/// `.AppImage` on Windows and handing it to `explorer`, which fails in a way
/// that looks like Emerald is broken rather than like the asset list changed.
fn wanted_asset(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        // The NSIS installer, not the .msi: it is the per-user one, matching
        // how this build was installed in the first place.
        return n.ends_with("-setup.exe");
    }
    if cfg!(target_os = "macos") {
        let arch_ok = if cfg!(target_arch = "aarch64") {
            n.contains("aarch64")
        } else {
            n.contains("x64") || n.contains("x86_64")
        };
        return n.ends_with(".dmg") && arch_ok;
    }
    // Linux: the AppImage runs anywhere, the .deb only on Debian-likes, and
    // nothing here can install a .deb without a package manager and a password.
    n.ends_with(".appimage")
}

/// Ask GitHub for the newest release. `Ok(None)` means "nothing newer".
pub async fn check(current: &str) -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent(AGENT)
        .build()
        .map_err(|e| format!("could not build the HTTP client: {e}"))?;

    let response = client
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned {}", response.status().as_u16()));
    }

    // `.text()` then parse, rather than reqwest's `json` feature: that feature
    // pulls in its own serde_json path for one call site that already has
    // serde_json available.
    let raw = response
        .text()
        .await
        .map_err(|e| format!("could not read GitHub's answer: {e}"))?;
    let body: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("GitHub's answer was not JSON: {e}"))?;

    // Pick the highest version rather than trusting the order the API returns.
    // GitHub sorts by creation date, which is not the same thing the moment a
    // patch for an older line is cut after a newer release.
    let releases = body.as_array().ok_or("GitHub did not return a release list")?;
    let Some(body) = releases
        .iter()
        .filter(|r| !r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false))
        .filter(|r| r.get("tag_name").and_then(|t| t.as_str()).is_some())
        .max_by(|a, b| {
            let tag = |r: &serde_json::Value| {
                r.get("tag_name")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let (x, y) = (tag(a), tag(b));
            if is_newer(&x, &y) {
                std::cmp::Ordering::Greater
            } else if is_newer(&y, &x) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
    else {
        return Ok(None); // no published releases at all
    };

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("that release has no tag")?;
    let version = tag.trim_start_matches('v').to_string();

    if !is_newer(&version, current) {
        return Ok(None);
    }

    let (asset_url, asset_name) = body
        .get("assets")
        .and_then(|a| a.as_array())
        .and_then(|assets| {
            assets.iter().find_map(|a| {
                let name = a.get("name")?.as_str()?;
                if !wanted_asset(name) {
                    return None;
                }
                let url = a.get("browser_download_url")?.as_str()?;
                Some((Some(url.to_string()), Some(name.to_string())))
            })
        })
        .unwrap_or((None, None));

    let notes = body
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(600)
        .collect::<String>();

    Ok(Some(UpdateInfo {
        version,
        current: current.to_string(),
        url: body
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/ZDStudios/Emerald/releases")
            .to_string(),
        notes,
        asset_url,
        asset_name,
    }))
}

/// Download an installer into `dir` and return where it landed.
///
/// The URL is checked against GitHub's release host rather than trusted,
/// because it arrives as a string from a network response and ends up as a
/// file this function then asks the OS to execute.
pub async fn download(url: &str, name: &str, dir: &PathBuf) -> Result<PathBuf, String> {
    let parsed = url::Url::parse(url).map_err(|_| "that download link is not a URL")?;
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() != "https"
        || !(host == "github.com" || host.ends_with(".github.com") || host == "objects.githubusercontent.com")
    {
        return Err("that download link does not point at GitHub".into());
    }
    // The filename comes from the release too, so it is never used as a path.
    let safe: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    if safe.is_empty() {
        return Err("that release asset has no usable filename".into());
    }

    let client = reqwest::Client::builder()
        .user_agent(AGENT)
        .build()
        .map_err(|e| format!("could not build the HTTP client: {e}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("the download could not start: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("the download returned {}", response.status().as_u16()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("the download did not finish: {e}"))?;

    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(&safe);
    std::fs::write(&path, &bytes).map_err(|e| format!("cannot save the installer: {e}"))?;

    // An AppImage is useless without the execute bit, and the browser that
    // downloaded it is the only thing that knows it is meant to be run.
    #[cfg(unix)]
    if safe.to_ascii_lowercase().ends_with(".appimage") {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically_not_lexically() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        // The one string comparison gets wrong, and gets wrong silently.
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("0.9.0", "0.10.0"));
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }

    #[test]
    fn shorter_versions_pad_rather_than_lose() {
        assert!(!is_newer("1", "1.0.0"));
        assert!(is_newer("1.0.1", "1"));
    }

    #[test]
    fn prerelease_suffixes_do_not_break_the_parse() {
        assert!(is_newer("0.2.0-beta.1", "0.1.0"));
        assert!(!is_newer("0.1.0-beta.1", "0.1.0"));
    }

    #[test]
    fn only_this_platforms_installer_is_chosen() {
        // Whatever the platform, an asset for a different one is never picked.
        assert!(!wanted_asset("Emerald_0.1.0_amd64.deb"));
        assert!(!wanted_asset("Emerald_x64.app.tar.gz"));
        assert!(!wanted_asset("emerald-source.zip"));

        let installers = [
            "Emerald_0.2.0_x64-setup.exe",
            "Emerald_0.2.0_aarch64.dmg",
            "Emerald_0.2.0_x64.dmg",
            "Emerald_0.2.0_amd64.AppImage",
        ];
        assert_eq!(
            installers.iter().filter(|n| wanted_asset(n)).count(),
            1,
            "exactly one asset should match the running platform"
        );
    }
}
