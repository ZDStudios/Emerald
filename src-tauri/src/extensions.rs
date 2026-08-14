//! Chrome extension installation and inventory.
//!
//! ## What actually works, per platform
//!
//! | Platform | Engine | Runs Chrome extensions? |
//! | --- | --- | --- |
//! | Windows | WebView2 | **Yes** — unpacked, from a folder |
//! | macOS | WKWebView | No — the API does not exist |
//! | Linux | WebKitGTK | No — its `extensions_path` takes compiled `.so` WebKit extensions, an unrelated technology with a confusingly similar name |
//!
//! Emerald not being Chromium on two of three platforms is the whole point of
//! the engine decision, and this is its sharpest cost. The code below is
//! therefore split deliberately: **installing and inventorying extensions is
//! cross-platform and fully tested**, because it is just files on disk, while
//! *loading* them into a webview only happens where the engine can. The
//! settings panel states the platform truth instead of showing a dead button.
//!
//! ## Why there is no Chrome Web Store button
//!
//! The Web Store's `.crx` endpoint serves Chrome-branded clients under terms
//! that do not cover third-party browsers, and it gates on the user agent.
//! Emerald could pretend to be Chrome and scrape it. It does not. What it
//! supports instead is the two routes Chrome itself offers in developer mode:
//! install a `.crx` file you downloaded, or point at an unpacked folder.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One extension found in the extensions directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledExtension {
    /// Directory name, used as the stable identifier.
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    /// `2` or `3`. MV2 is dead in Chrome and is flagged in the UI.
    pub manifest_version: u32,
    /// Permissions the manifest requests, shown before enabling.
    pub permissions: Vec<String>,
    /// Host match patterns, which are the ones that actually matter.
    pub host_permissions: Vec<String>,
    pub enabled: bool,
    pub path: String,
}

/// Read every extension in `dir`.
///
/// A folder without a readable `manifest.json` is skipped rather than being
/// reported as broken — the directory is user-visible and may contain
/// anything.
pub fn list(dir: &Path, disabled: &[String]) -> Vec<InstalledExtension> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<InstalledExtension> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            read_manifest(&e.path(), &id, !disabled.contains(&id))
        })
        .collect();
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

fn read_manifest(path: &Path, id: &str, enabled: bool) -> Option<InstalledExtension> {
    let raw = std::fs::read_to_string(path.join("manifest.json")).ok()?;
    // Chrome tolerates comments in manifests; serde_json does not. Strip
    // line comments so an extension that loads in Chrome loads here too.
    let cleaned = strip_line_comments(&raw);
    let manifest: serde_json::Value = serde_json::from_str(&cleaned).ok()?;

    let str_at = |key: &str| {
        manifest
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let list_at = |key: &str| {
        manifest
            .get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    let name = match str_at("name") {
        n if n.is_empty() => id.to_string(),
        n => n,
    };

    Some(InstalledExtension {
        id: id.to_string(),
        name,
        version: str_at("version"),
        description: str_at("description"),
        manifest_version: manifest
            .get("manifest_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(2) as u32,
        permissions: list_at("permissions"),
        host_permissions: list_at("host_permissions"),
        enabled,
        path: path.to_string_lossy().to_string(),
    })
}

/// Remove `//` comments that are not inside a string literal.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Install from a `.crx` (or a plain `.zip`) into `dest_dir`.
///
/// Returns the installed extension. The archive's signature is **not**
/// verified: Emerald has no Web Store public key to check against, and a
/// signature by an unknown party proves nothing. The security boundary here is
/// the permission prompt in the settings panel, which shows what the manifest
/// asks for before the extension is enabled.
pub fn install_archive(archive: &Path, dest_dir: &Path) -> Result<InstalledExtension, String> {
    let bytes = std::fs::read(archive).map_err(|e| format!("cannot read archive: {e}"))?;
    let zip_start = crx_payload_offset(&bytes)?;
    let cursor = std::io::Cursor::new(&bytes[zip_start..]);
    let mut zip =
        zip::ZipArchive::new(cursor).map_err(|e| format!("not a valid extension package: {e}"))?;

    let id = derive_id(archive, dest_dir);
    let target = dest_dir.join(&id);
    std::fs::create_dir_all(&target).map_err(|e| format!("cannot create {id}: {e}"))?;

    zip.extract(&target).map_err(|e| {
        let _ = std::fs::remove_dir_all(&target);
        format!("cannot unpack extension: {e}")
    })?;

    // Some packages nest everything one level down. Accept that shape rather
    // than failing with "no manifest" on a package that Chrome would load.
    let root = locate_manifest_root(&target);
    read_manifest(&root, &id, true).ok_or_else(|| {
        let _ = std::fs::remove_dir_all(&target);
        "package has no readable manifest.json".to_string()
    })
}

/// Byte offset where the ZIP payload begins.
///
/// `.crx` is a header followed by an ordinary ZIP. Three shapes exist and all
/// three are still in the wild:
///   * CRX3 — `Cr24`, version 3, u32 header length, protobuf header
///   * CRX2 — `Cr24`, version 2, u32 key length, u32 signature length
///   * a plain `.zip`, which unpacked extensions are often distributed as
fn crx_payload_offset(bytes: &[u8]) -> Result<usize, String> {
    if bytes.starts_with(b"PK\x03\x04") {
        return Ok(0);
    }
    if !bytes.starts_with(b"Cr24") {
        return Err("not a .crx or .zip package".into());
    }
    let u32_at = |i: usize| -> Result<usize, String> {
        bytes
            .get(i..i + 4)
            .and_then(|s| s.try_into().ok())
            .map(|b: [u8; 4]| u32::from_le_bytes(b) as usize)
            .ok_or_else(|| "truncated .crx header".to_string())
    };
    let offset = match u32_at(4)? {
        3 => 12 + u32_at(8)?,
        2 => 16 + u32_at(8)? + u32_at(12)?,
        v => return Err(format!("unsupported .crx version {v}")),
    };
    if offset >= bytes.len() {
        return Err("truncated .crx package".into());
    }
    Ok(offset)
}

/// A directory name that is stable, filesystem-safe, and not already taken.
fn derive_id(archive: &Path, dest_dir: &Path) -> String {
    let stem = archive
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "extension".into());
    let base: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let base = base.trim_matches('-').to_lowercase();
    let base = if base.is_empty() { "extension".to_string() } else { base };

    if !dest_dir.join(&base).exists() {
        return base;
    }
    (2..1000)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !dest_dir.join(candidate).exists())
        .unwrap_or(base)
}

/// Find the directory actually containing `manifest.json`, allowing one level
/// of nesting.
fn locate_manifest_root(dir: &Path) -> PathBuf {
    if dir.join("manifest.json").exists() {
        return dir.to_path_buf();
    }
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir() && p.join("manifest.json").exists())
        .unwrap_or_else(|| dir.to_path_buf())
}

/// Delete an installed extension. Refuses anything that is not a plain name,
/// so a crafted id cannot escape the extensions directory.
pub fn remove(dir: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("invalid extension id".into());
    }
    let target = dir.join(id);
    if !target.is_dir() {
        return Err("no such extension".into());
    }
    std::fs::remove_dir_all(&target).map_err(|e| format!("cannot remove: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emerald-ext-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A minimal but real extension package.
    fn make_zip(manifest: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            w.start_file::<_, ()>("manifest.json", Default::default()).unwrap();
            w.write_all(manifest.as_bytes()).unwrap();
            w.finish().unwrap();
        }
        buf
    }

    const MANIFEST: &str = r#"{
      "manifest_version": 3,
      "name": "Test Extension",
      "version": "1.2.3",
      "description": "Does a thing",
      "permissions": ["storage", "tabs"],
      "host_permissions": ["https://*.example.com/*"]
    }"#;

    #[test]
    fn installs_a_plain_zip_package() {
        let dir = tmp("zip");
        let pkg = dir.join("my-ext.zip");
        std::fs::write(&pkg, make_zip(MANIFEST)).unwrap();
        let dest = dir.join("installed");
        std::fs::create_dir_all(&dest).unwrap();

        let ext = install_archive(&pkg, &dest).unwrap();
        assert_eq!(ext.name, "Test Extension");
        assert_eq!(ext.version, "1.2.3");
        assert_eq!(ext.manifest_version, 3);
        assert_eq!(ext.permissions, vec!["storage", "tabs"]);
        assert_eq!(ext.host_permissions, vec!["https://*.example.com/*"]);
        assert!(dest.join("my-ext/manifest.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installs_a_crx3_package() {
        let dir = tmp("crx3");
        // Cr24 | version 3 | header length | header | zip
        let header = vec![0u8; 24];
        let mut crx = Vec::new();
        crx.extend_from_slice(b"Cr24");
        crx.extend_from_slice(&3u32.to_le_bytes());
        crx.extend_from_slice(&(header.len() as u32).to_le_bytes());
        crx.extend_from_slice(&header);
        crx.extend_from_slice(&make_zip(MANIFEST));

        let pkg = dir.join("packed.crx");
        std::fs::write(&pkg, crx).unwrap();
        let dest = dir.join("installed");
        std::fs::create_dir_all(&dest).unwrap();

        let ext = install_archive(&pkg, &dest).unwrap();
        assert_eq!(ext.name, "Test Extension");
        assert_eq!(ext.id, "packed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installs_a_crx2_package() {
        let dir = tmp("crx2");
        let (key, sig) = (vec![1u8; 16], vec![2u8; 32]);
        let mut crx = Vec::new();
        crx.extend_from_slice(b"Cr24");
        crx.extend_from_slice(&2u32.to_le_bytes());
        crx.extend_from_slice(&(key.len() as u32).to_le_bytes());
        crx.extend_from_slice(&(sig.len() as u32).to_le_bytes());
        crx.extend_from_slice(&key);
        crx.extend_from_slice(&sig);
        crx.extend_from_slice(&make_zip(MANIFEST));

        let pkg = dir.join("old.crx");
        std::fs::write(&pkg, crx).unwrap();
        let dest = dir.join("installed");
        std::fs::create_dir_all(&dest).unwrap();
        assert_eq!(install_archive(&pkg, &dest).unwrap().version, "1.2.3");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_junk_without_panicking() {
        assert!(crx_payload_offset(b"not an extension at all").is_err());
        assert!(crx_payload_offset(b"Cr24").is_err(), "truncated header");
        // Claims a 4GB header in a 20-byte file.
        let mut lying = b"Cr24".to_vec();
        lying.extend_from_slice(&3u32.to_le_bytes());
        lying.extend_from_slice(&u32::MAX.to_le_bytes());
        lying.extend_from_slice(&[0; 8]);
        assert!(crx_payload_offset(&lying).is_err());
    }

    #[test]
    fn manifests_with_comments_still_load() {
        // Chrome accepts these; a strict JSON parser does not.
        let with_comments = r#"{
          // the name shown in the UI
          "manifest_version": 3,
          "name": "Commented",
          "version": "1.0",
          "description": "https://not-a-comment.example/x"
        }"#;
        let dir = tmp("comments");
        std::fs::create_dir_all(dir.join("ext")).unwrap();
        std::fs::write(dir.join("ext/manifest.json"), with_comments).unwrap();

        let ext = read_manifest(&dir.join("ext"), "ext", true).unwrap();
        assert_eq!(ext.name, "Commented");
        // A `//` inside a string is not a comment.
        assert_eq!(ext.description, "https://not-a-comment.example/x");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn listing_skips_folders_without_a_manifest_and_honours_disabled() {
        let dir = tmp("list");
        for name in ["alpha", "beta"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            std::fs::write(
                dir.join(name).join("manifest.json"),
                format!(r#"{{"manifest_version":3,"name":"{name}","version":"1"}}"#),
            )
            .unwrap();
        }
        std::fs::create_dir_all(dir.join("junk")).unwrap();

        let found = list(&dir, &["beta".to_string()]);
        assert_eq!(found.len(), 2, "junk folder ignored: {found:?}");
        assert!(found[0].enabled, "alpha enabled");
        assert!(!found[1].enabled, "beta disabled");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_refuses_to_escape_the_extensions_directory() {
        let dir = tmp("escape");
        assert!(remove(&dir, "../../etc").is_err());
        assert!(remove(&dir, "a/b").is_err());
        assert!(remove(&dir, "").is_err());
        assert!(remove(&dir, "nonexistent").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installing_twice_does_not_overwrite_the_first() {
        let dir = tmp("dupe");
        let pkg = dir.join("dupe.zip");
        std::fs::write(&pkg, make_zip(MANIFEST)).unwrap();
        let dest = dir.join("installed");
        std::fs::create_dir_all(&dest).unwrap();

        let a = install_archive(&pkg, &dest).unwrap();
        let b = install_archive(&pkg, &dest).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(b.id, "dupe-2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
