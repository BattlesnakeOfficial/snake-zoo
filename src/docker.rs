use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Verify docker is on PATH and the daemon is running.
pub fn check_docker() -> Result<()> {
    let version = Command::new("docker")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match version {
        Ok(s) if s.success() => {}
        _ => bail!("docker not found on PATH — install Docker and try again"),
    }

    let info = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to run docker info")?;

    if !info.success() {
        bail!("Docker daemon is not running — start Docker and try again");
    }

    Ok(())
}

/// Check whether a Docker image exists locally.
pub fn image_exists(image_name: &str) -> Result<bool> {
    let status = Command::new("docker")
        .args(["image", "inspect", image_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to run docker image inspect")?;

    Ok(status.success())
}

/// Build a Docker image from a Dockerfile in the given build context directory.
pub fn build_image(image_tag: &str, dockerfile: &str, build_context: &Path) -> Result<()> {
    let status = Command::new("docker")
        .args(["build", "-t", image_tag, "-f", dockerfile, "."])
        .current_dir(build_context)
        .status()
        .context("failed to run docker build")?;

    if !status.success() {
        bail!("docker build failed for {image_tag}");
    }

    Ok(())
}

/// Tag an existing image with a new name.
pub fn tag_image(source: &str, target: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["tag", source, target])
        .status()
        .context("failed to run docker tag")?;

    if !status.success() {
        bail!("docker tag failed: {source} -> {target}");
    }

    Ok(())
}

/// Run a container in detached mode with a random host port mapped to container_port.
/// Returns the container ID.
pub fn run_container(
    container_name: &str,
    image_tag: &str,
    container_port: u16,
    env_vars: &HashMap<String, String>,
) -> Result<String> {
    let port_mapping = format!("0:{container_port}");

    let mut cmd = Command::new("docker");
    cmd.args(["run", "-d", "--name", container_name, "-p", &port_mapping]);

    for (key, value) in env_vars {
        cmd.args(["--env", &format!("{key}={value}")]);
    }

    cmd.arg(image_tag);

    let output = cmd.output().context("failed to run docker run")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker run failed for {image_tag}: {stderr}");
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(container_id)
}

/// Get the host port mapped to a container port.
pub fn get_host_port(container_name: &str, container_port: u16) -> Result<u16> {
    let output = Command::new("docker")
        .args(["port", container_name, &format!("{container_port}/tcp")])
        .output()
        .context("failed to run docker port")?;

    if !output.status.success() {
        bail!("docker port failed for {container_name}:{container_port}/tcp");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output may have multiple lines (IPv4 + IPv6). Take the first line.
    let first_line = stdout.lines().next().context("empty docker port output")?;

    // Format is either "0.0.0.0:49153" or ":::49153"
    let port_str = first_line
        .rsplit(':')
        .next()
        .context("unexpected docker port output format")?;

    port_str
        .trim()
        .parse::<u16>()
        .with_context(|| format!("failed to parse port number from '{port_str}'"))
}

/// Stop and remove a container (best-effort, ignores errors).
pub fn stop_and_remove(container_name: &str) {
    let _ = Command::new("docker")
        .args(["stop", "-t", "5", container_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let _ = Command::new("docker")
        .args(["rm", container_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Force-remove a container if it exists (handles both running and stopped).
pub fn remove_if_exists(container_name: &str) {
    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Follow container logs, returning the child process with piped stdout/stderr.
pub fn follow_logs(container_name: &str) -> Result<Child> {
    Command::new("docker")
        .args(["logs", "--follow", "--timestamps", container_name])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to follow logs for {container_name}"))
}
