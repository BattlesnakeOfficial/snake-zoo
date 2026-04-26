mod docker;
mod git;
mod manifest;
mod run;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "snake-zoo",
    about = "Run Battlesnake opponents locally with Docker"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available snakes and their build status
    List,
    /// Build and run snakes
    Run {
        /// Snake slugs to run
        names: Vec<String>,
        /// Run all snakes
        #[arg(long, conflicts_with = "names")]
        all: bool,
    },
}

fn cmd_list(manifests: &[manifest::SnakeManifest]) -> Result<()> {
    let name_header = "Name";
    let slug_header = "Slug";
    let repo_header = "Repo";
    let built_header = "Built";

    let max_name = manifests
        .iter()
        .map(|m| m.name.len())
        .max()
        .unwrap_or(0)
        .max(name_header.len());

    let max_slug = manifests
        .iter()
        .map(|m| m.slug.len())
        .max()
        .unwrap_or(0)
        .max(slug_header.len());

    let max_repo = manifests
        .iter()
        .map(|m| m.repo.len())
        .max()
        .unwrap_or(0)
        .max(repo_header.len());

    println!(
        "{:<max_name$}  {:<max_slug$}  {:<max_repo$}  {built_header}",
        name_header, slug_header, repo_header
    );
    println!(
        "{:<max_name$}  {:<max_slug$}  {:<max_repo$}  {}",
        "-".repeat(max_name),
        "-".repeat(max_slug),
        "-".repeat(max_repo),
        "-".repeat(built_header.len())
    );

    for m in manifests {
        let image_name = format!("snake-zoo/{}", m.slug);
        let built = match docker::image_exists(&image_name) {
            Ok(true) => "yes",
            _ => "no",
        };
        println!(
            "{:<max_name$}  {:<max_slug$}  {:<max_repo$}  {built}",
            m.name, m.slug, m.repo
        );
    }

    if manifests.iter().any(|m| !m.description.is_empty()) {
        println!();
        for m in manifests.iter().filter(|m| !m.description.is_empty()) {
            println!("  {}: {}", m.slug, m.description);
        }
    }

    Ok(())
}

fn cmd_run(manifests: &[manifest::SnakeManifest], names: &[String], all: bool) -> Result<()> {
    if !all && names.is_empty() {
        bail!("specify at least one snake name, or use --all");
    }

    let selected: Vec<&manifest::SnakeManifest> = if all {
        manifests.iter().collect()
    } else {
        let mut selected = Vec::new();
        let mut unknown = Vec::new();

        for name in names {
            match manifests.iter().find(|m| m.slug == *name) {
                Some(m) => selected.push(m),
                None => unknown.push(name.as_str()),
            }
        }

        if !unknown.is_empty() {
            bail!("unknown snake(s): {}", unknown.join(", "));
        }

        selected
    };

    docker::check_docker()?;

    // Collect owned copies for the run function
    let owned: Vec<manifest::SnakeManifest> = selected.into_iter().cloned().collect();

    run::run(&owned)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let manifests = manifest::load_manifests()?;

    match &cli.command {
        Commands::List => cmd_list(&manifests),
        Commands::Run { names, all } => cmd_run(&manifests, names, *all),
    }
}
