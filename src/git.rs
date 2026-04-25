use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_dir_name(repo_url: &str) -> String {
    let trimmed = repo_url.trim_end_matches('/');
    let without_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let segments: Vec<&str> = without_git.split('/').filter(|s| !s.is_empty()).collect();
    match segments.len() {
        0 => "unknown".to_string(),
        1 => segments[0].to_string(),
        _ => format!(
            "{}-{}",
            segments[segments.len() - 2],
            segments[segments.len() - 1]
        ),
    }
}

pub fn clone_or_pull(repo_url: &str, cache_dir: &Path) -> Result<PathBuf> {
    let dir_name = repo_dir_name(repo_url);
    let repo_path = cache_dir.join(&dir_name);

    if repo_path.exists() && repo_path.join(".git").exists() {
        eprintln!("Updating {dir_name}...");
        let status = Command::new("git")
            .args(["-C", &repo_path.to_string_lossy(), "pull", "--ff-only"])
            .status()
            .context("failed to run git pull")?;
        if !status.success() {
            bail!("git pull failed for {repo_url} in {}", repo_path.display());
        }
    } else {
        eprintln!("Cloning {repo_url}...");
        let status = Command::new("git")
            .args(["clone", repo_url, &repo_path.to_string_lossy()])
            .status()
            .context("failed to run git clone")?;
        if !status.success() {
            bail!("git clone failed for {repo_url}");
        }
    }

    Ok(repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_dir_name_standard() {
        assert_eq!(
            repo_dir_name("https://github.com/coreyja/battlesnake-rs"),
            "coreyja-battlesnake-rs"
        );
    }

    #[test]
    fn test_repo_dir_name_with_git_suffix() {
        assert_eq!(
            repo_dir_name("https://github.com/coreyja/battlesnake-rs.git"),
            "coreyja-battlesnake-rs"
        );
    }

    #[test]
    fn test_repo_dir_name_trailing_slash() {
        assert_eq!(
            repo_dir_name("https://github.com/coreyja/battlesnake-rs/"),
            "coreyja-battlesnake-rs"
        );
    }

    #[test]
    fn test_repo_dir_name_different_orgs() {
        let alice = repo_dir_name("https://github.com/alice/snake");
        let bob = repo_dir_name("https://github.com/bob/snake");
        assert_eq!(alice, "alice-snake");
        assert_eq!(bob, "bob-snake");
        assert_ne!(alice, bob);
    }
}
