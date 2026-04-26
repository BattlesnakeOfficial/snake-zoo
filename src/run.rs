use crate::manifest::SnakeManifest;
use crate::{docker, git};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc;
use std::thread;

/// Resolve the cache directory for cloned repos.
///
/// Uses the platform cache directory (XDG_CACHE_HOME on Linux, ~/Library/Caches on
/// macOS, %LOCALAPPDATA% on Windows) so the clone cache is shared across checkouts
/// of this repo rather than tied to the current working directory.
fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .context("could not determine cache directory (set XDG_CACHE_HOME or HOME on Linux)")?;
    Ok(base.join("snake-zoo").join("repos"))
}

struct RunningSnake {
    slug: String,
    container_name: String,
    host_port: u16,
    entrypoint: String,
}

impl RunningSnake {
    fn url(&self) -> String {
        let entrypoint = self.entrypoint.trim_start_matches('/');
        if entrypoint.is_empty() {
            format!("http://localhost:{}", self.host_port)
        } else {
            format!("http://localhost:{}/{entrypoint}", self.host_port)
        }
    }
}

/// Clone or pull repos for all snakes, deduplicating by repo URL.
fn clone_repos(snakes: &[SnakeManifest], cache_dir: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut repo_paths: HashMap<String, PathBuf> = HashMap::new();

    for snake in snakes {
        if repo_paths.contains_key(&snake.repo) {
            continue;
        }
        let path = git::clone_or_pull(&snake.repo, cache_dir)?;
        repo_paths.insert(snake.repo.clone(), path);
    }

    Ok(repo_paths)
}

/// Build Docker images, deduplicating by (repo, dockerfile) tuple.
fn build_images(snakes: &[SnakeManifest], repo_paths: &HashMap<String, PathBuf>) -> Result<()> {
    // Group snakes by (repo, dockerfile)
    let mut groups: HashMap<(String, String), Vec<&SnakeManifest>> = HashMap::new();
    for snake in snakes {
        let key = (snake.repo.clone(), snake.dockerfile.clone());
        groups.entry(key).or_default().push(snake);
    }

    for ((_repo, _dockerfile), group_snakes) in &groups {
        let first = group_snakes[0];
        let canonical_tag = format!("snake-zoo/{}", first.slug);
        let build_context = repo_paths.get(&first.repo).context("missing repo path")?;

        if !docker::image_exists(&canonical_tag)? {
            eprintln!("Building {canonical_tag}...");
            docker::build_image(&canonical_tag, &first.dockerfile, build_context)?;
        } else {
            eprintln!("Image {canonical_tag} already exists, skipping build.");
        }

        // Tag for other snakes in the same group
        for snake in &group_snakes[1..] {
            let tag = format!("snake-zoo/{}", snake.slug);
            if !docker::image_exists(&tag)? {
                docker::tag_image(&canonical_tag, &tag)?;
            }
        }
    }

    Ok(())
}

/// Start containers for all snakes, cleaning up on partial failure.
fn start_containers(snakes: &[SnakeManifest]) -> Result<Vec<RunningSnake>> {
    let mut running_snakes = Vec::new();

    for snake in snakes {
        let container_name = format!("snake-zoo-{}", snake.slug);
        docker::remove_if_exists(&container_name);

        let _container_id = match docker::run_container(
            &container_name,
            &format!("snake-zoo/{}", snake.slug),
            snake.port,
            &snake.env,
        ) {
            Ok(id) => id,
            Err(e) => {
                cleanup(&running_snakes);
                bail!("failed to start container for {}: {e}", snake.slug);
            }
        };

        let host_port = match docker::get_host_port(&container_name, snake.port) {
            Ok(port) => port,
            Err(e) => {
                // This container is running but not yet in running_snakes — stop it explicitly
                docker::stop_and_remove(&container_name);
                cleanup(&running_snakes);
                bail!("failed to get port for {}: {e}", snake.slug);
            }
        };

        running_snakes.push(RunningSnake {
            slug: snake.slug.clone(),
            container_name,
            host_port,
            entrypoint: snake.entrypoint.clone(),
        });
    }

    Ok(running_snakes)
}

/// Print a formatted table of snake slugs and URLs.
fn print_url_table(snakes: &[RunningSnake]) {
    let slug_header = "Snake";
    let url_header = "URL";

    let max_slug = snakes
        .iter()
        .map(|s| s.slug.len())
        .max()
        .unwrap_or(0)
        .max(slug_header.len());

    println!("{:<max_slug$}  {url_header}", slug_header);
    println!(
        "{:<max_slug$}  {}",
        "-".repeat(max_slug),
        "-".repeat(url_header.len())
    );

    for snake in snakes {
        println!("{:<max_slug$}  {}", snake.slug, snake.url());
    }
}

/// Follow container logs and block until Ctrl+C.
fn follow_and_wait(running_snakes: &[RunningSnake]) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })
    .context("failed to set Ctrl+C handler")?;

    let mut log_children: Vec<Child> = Vec::new();

    for snake in running_snakes {
        let mut log_child = match docker::follow_logs(&snake.container_name) {
            Ok(child) => child,
            Err(e) => {
                for mut child in log_children {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                return Err(e);
            }
        };

        let stdout = log_child.stdout.take().unwrap();
        let stderr = log_child.stderr.take().unwrap();
        let slug = snake.slug.clone();

        thread::Builder::new()
            .name(format!("log-{slug}-stdout"))
            .spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => println!("[{slug}] {line}"),
                        Err(_) => break,
                    }
                }
            })
            .expect("failed to spawn log thread");

        let slug_stderr = snake.slug.clone();
        thread::Builder::new()
            .name(format!("log-{slug_stderr}-stderr"))
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line) => println!("[{slug_stderr}] {line}"),
                        Err(_) => break,
                    }
                }
            })
            .expect("failed to spawn log thread");

        log_children.push(log_child);
    }

    // Block until Ctrl+C
    let _ = shutdown_rx.recv();

    // Kill log followers and reap zombies
    for mut child in log_children {
        let _ = child.kill();
        let _ = child.wait();
    }

    Ok(())
}

/// Stop and remove all running containers (best-effort).
fn cleanup(running_snakes: &[RunningSnake]) {
    eprintln!("\nShutting down...");
    for snake in running_snakes {
        docker::stop_and_remove(&snake.container_name);
    }
}

/// Main run orchestration: clone repos, build images, start containers, follow logs.
pub fn run(snakes: &[SnakeManifest]) -> Result<()> {
    let cache_dir = cache_dir()?;
    std::fs::create_dir_all(&cache_dir).with_context(|| {
        format!(
            "failed to create cache directory at {}",
            cache_dir.display()
        )
    })?;

    // Phase 1: Clone/pull repos
    let repo_paths = clone_repos(snakes, &cache_dir)?;

    // Phase 2: Build images (no containers yet)
    build_images(snakes, &repo_paths)?;

    // Phase 3: Start containers (handles its own cleanup on partial failure)
    let running_snakes = start_containers(snakes)?;

    // Phase 4: Print URL table
    print_url_table(&running_snakes);

    // Phase 5: Follow logs and wait for Ctrl+C
    let result = follow_and_wait(&running_snakes);

    // Phase 6: Cleanup (always runs)
    cleanup(&running_snakes);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_snake_url_with_entrypoint() {
        let snake = RunningSnake {
            slug: "my-snake".to_string(),
            container_name: "snake-zoo-my-snake".to_string(),
            host_port: 8080,
            entrypoint: "my-snake".to_string(),
        };
        assert_eq!(snake.url(), "http://localhost:8080/my-snake");
    }

    #[test]
    fn test_running_snake_url_no_entrypoint() {
        let snake = RunningSnake {
            slug: "my-snake".to_string(),
            container_name: "snake-zoo-my-snake".to_string(),
            host_port: 8080,
            entrypoint: String::new(),
        };
        assert_eq!(snake.url(), "http://localhost:8080");
    }

    #[test]
    fn test_running_snake_url_leading_slash_entrypoint() {
        let snake = RunningSnake {
            slug: "my-snake".to_string(),
            container_name: "snake-zoo-my-snake".to_string(),
            host_port: 8080,
            entrypoint: "/my-snake".to_string(),
        };
        assert_eq!(snake.url(), "http://localhost:8080/my-snake");
    }
}
