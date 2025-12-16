mod clipboard;
mod config;
mod gather;

use anyhow::{Context, Result};
use clap::Parser;
use config::ConfigFile;
use gather::{collect_from_path, collect_from_preset, render_files};
use std::path::{Path, PathBuf};
use std::time::Instant;

const CONFIG_FILE_NAME: &str = ".gather-files.yaml";

#[derive(Parser, Debug)]
#[command(name = "gf")]
#[command(about = "Gather files, stitch them together, and copy contents to the clipboard.")]
struct Cli {
    /// Optional target (directory path or preset name)
    target: Option<String>,
    /// Path to config file (.gather-files.yaml)
    #[arg(long, default_value = CONFIG_FILE_NAME)]
    config: String,
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let start = Instant::now();
    let cli = Cli::parse();
    let current_dir =
        std::env::current_dir().context("failed to determine current working directory")?;
    let repo_root = find_repo_root(&current_dir).unwrap_or(current_dir.clone());
    let config_path = resolve_config_path(&repo_root, &cli.config);
    let config = ConfigFile::load(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;

    let (files, description) = determine_target(&cli.target, &repo_root, config.as_ref())?;

    if files.is_empty() {
        println!("No files found for {}.", description);
        return Ok(());
    }

    let (rendered, char_count) = render_files(&files, &repo_root)?;
    clipboard::copy_to_clipboard(&rendered)?;

    let elapsed = start.elapsed();
    println!(
        "Copied {} chars from {} files ({}) in {:.2?}.",
        char_count,
        files.len(),
        description,
        elapsed
    );

    Ok(())
}

fn determine_target(
    target: &Option<String>,
    repo_root: &Path,
    config: Option<&ConfigFile>,
) -> Result<(Vec<PathBuf>, String)> {
    match target {
        None => {
            let files = collect_from_path(repo_root)?;
            Ok((files, format!("root {}", repo_root.display())))
        }
        Some(argument) => {
            let path_candidate = parse_target_path(argument, repo_root);
            if path_candidate.exists() {
                let files = collect_from_path(&path_candidate)?;
                return Ok((files, format!("path {}", path_candidate.display())));
            }

            let config = config.ok_or_else(|| {
                anyhow::anyhow!("no config found when looking for preset '{argument}'")
            })?;
            let preset = config
                .preset(argument)
                .ok_or_else(|| anyhow::anyhow!("preset '{argument}' not found in config"))?;
            let files = collect_from_preset(argument, preset, repo_root)?;
            Ok((files, format!("preset '{argument}'")))
        }
    }
}

fn parse_target_path(argument: &str, repo_root: &Path) -> PathBuf {
    let path = Path::new(argument);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn resolve_config_path(repo_root: &Path, config: &str) -> PathBuf {
    let path = Path::new(config);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}
