use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SnakeManifest {
    pub name: String,
    pub slug: String,
    pub repo: String,
    #[serde(default = "default_dockerfile")]
    pub dockerfile: String,
    #[serde(default)]
    pub entrypoint: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub meta: Option<SnakeMeta>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Metadata about the snake's appearance and strategy.
/// These fields are defined by the manifest format (SZ-04c3) and deserialized
/// from TOML. They will be consumed by future commands (e.g., display, info).
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SnakeMeta {
    pub author: Option<String>,
    pub strategy: Option<String>,
    pub color: Option<String>,
    pub head: Option<String>,
    pub tail: Option<String>,
}

fn default_dockerfile() -> String {
    "./Dockerfile".to_string()
}

fn default_port() -> u16 {
    8000
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn load_manifests(snakes_dir: &Path) -> Result<Vec<SnakeManifest>> {
    let entries = std::fs::read_dir(snakes_dir)
        .with_context(|| format!("failed to read snakes directory: {}", snakes_dir.display()))?;

    let mut manifests = Vec::new();

    for entry in entries {
        let entry = entry.context("failed to read directory entry")?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let manifest: SnakeManifest = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        if !is_valid_slug(&manifest.slug) {
            bail!(
                "{}: invalid slug '{}' — must match [a-z0-9][a-z0-9-]*",
                path.display(),
                manifest.slug
            );
        }

        let expected_slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if manifest.slug != expected_slug {
            bail!(
                "{}: slug '{}' does not match filename (expected '{}')",
                path.display(),
                manifest.slug,
                expected_slug
            );
        }

        manifests.push(manifest);
    }

    // Check for duplicate slugs
    let mut seen = std::collections::HashSet::new();
    for m in &manifests {
        if !seen.insert(&m.slug) {
            bail!("duplicate slug: '{}'", m.slug);
        }
    }

    manifests.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_full_manifest() {
        let toml_str = r##"
name = "Amphibious Arthur"
slug = "amphibious-arthur"
repo = "https://github.com/coreyja/battlesnake-rs"
dockerfile = "./Dockerfile"
entrypoint = "amphibious-arthur"
port = 8000
description = "Simulates opponent sprawl."

[meta]
author = "coreyja"
strategy = "Recursive Simulation"
color = "#AA66CC"
head = "trans-rights-scarf"
tail = "swirl"

[env]
RUST_LOG = "info"
"##;
        let manifest: SnakeManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "Amphibious Arthur");
        assert_eq!(manifest.slug, "amphibious-arthur");
        assert_eq!(manifest.repo, "https://github.com/coreyja/battlesnake-rs");
        assert_eq!(manifest.dockerfile, "./Dockerfile");
        assert_eq!(manifest.entrypoint, "amphibious-arthur");
        assert_eq!(manifest.port, 8000);
        assert_eq!(manifest.description, "Simulates opponent sprawl.");

        let meta = manifest.meta.unwrap();
        assert_eq!(meta.author.unwrap(), "coreyja");
        assert_eq!(meta.strategy.unwrap(), "Recursive Simulation");
        assert_eq!(meta.color.unwrap(), "#AA66CC");
        assert_eq!(meta.head.unwrap(), "trans-rights-scarf");
        assert_eq!(meta.tail.unwrap(), "swirl");

        assert_eq!(manifest.env.get("RUST_LOG").unwrap(), "info");
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let toml_str = r#"
name = "Simple Snake"
slug = "simple-snake"
repo = "https://github.com/example/snake"
"#;
        let manifest: SnakeManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "Simple Snake");
        assert_eq!(manifest.slug, "simple-snake");
        assert_eq!(manifest.dockerfile, "./Dockerfile");
        assert_eq!(manifest.port, 8000);
        assert_eq!(manifest.entrypoint, "");
        assert!(manifest.meta.is_none());
        assert!(manifest.env.is_empty());
    }

    #[test]
    fn test_load_manifests_from_dir() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("beta-snake.toml"),
            r#"
name = "Beta Snake"
slug = "beta-snake"
repo = "https://github.com/example/beta"
"#,
        )
        .unwrap();

        fs::write(
            dir.path().join("alpha-snake.toml"),
            r#"
name = "Alpha Snake"
slug = "alpha-snake"
repo = "https://github.com/example/alpha"
"#,
        )
        .unwrap();

        let manifests = load_manifests(dir.path()).unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].slug, "alpha-snake");
        assert_eq!(manifests[1].slug, "beta-snake");
    }

    #[test]
    fn test_slug_filename_mismatch() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("wrong-name.toml"),
            r#"
name = "Snake"
slug = "actual-slug"
repo = "https://github.com/example/snake"
"#,
        )
        .unwrap();

        let result = load_manifests(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not match filename"), "got: {err}");
    }

    #[test]
    fn test_invalid_slug_chars() {
        assert!(!is_valid_slug("Has Spaces"));
        assert!(!is_valid_slug("UPPERCASE"));
        assert!(!is_valid_slug("special@char"));
        assert!(!is_valid_slug("-starts-with-dash"));
        assert!(!is_valid_slug(""));
    }

    #[test]
    fn test_valid_slug() {
        assert!(is_valid_slug("my-snake-42"));
        assert!(is_valid_slug("a"));
        assert!(is_valid_slug("snake"));
        assert!(is_valid_slug("42-snake"));
    }
}
