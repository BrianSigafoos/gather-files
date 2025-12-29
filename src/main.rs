mod clipboard;
mod config;
mod gather;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::ConfigFile;
use gather::{collect_from_path, collect_from_preset, render_files};
use std::fs::OpenOptions;
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::time::Instant;

const CONFIG_FILE_NAME: &str = ".gather-files.yaml";

/// GitHub repository for releases
const GITHUB_REPO: &str = "BrianSigafoos/gather-files";

/// Install script URL
const INSTALL_SCRIPT_URL: &str = "https://gf.bfoos.net/install.sh";

#[derive(Parser, Debug)]
#[command(name = "gf")]
#[command(version)]
#[command(about = "Gather files, stitch them together, and copy contents to the clipboard.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Optional target (directory path or preset name)
    target: Option<String>,

    /// Path to config file (.gather-files.yaml)
    #[arg(long, default_value = CONFIG_FILE_NAME)]
    config: String,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a starter .gather-files.yaml config file
    Init,
    /// Update gf to the latest version
    Upgrade {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init) => {
            run_init()?;
            return Ok(());
        }
        Some(Command::Upgrade { check }) => {
            run_upgrade(check)?;
            return Ok(());
        }
        None => {}
    }

    let start = Instant::now();
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

fn run_init() -> Result<()> {
    let config_path = Path::new(CONFIG_FILE_NAME);

    if config_path.exists() {
        println!("Config file already exists at {}", config_path.display());
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(config_path)
        .with_context(|| format!("failed to create {}", config_path.display()))?;

    file.write_all(CONFIG_TEMPLATE.as_bytes())
        .context("failed to write config template")?;

    println!(
        "Created {}. Edit the presets to match your project.",
        config_path.display()
    );

    Ok(())
}

fn run_upgrade(check_only: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: v{}", current_version);

    print!("Checking for updates... ");
    let _ = stdout().flush();

    let latest_version = fetch_latest_version().context("failed to check for updates")?;
    println!("latest is v{}", latest_version);

    if is_newer_version(&latest_version, current_version) {
        println!();
        if check_only {
            println!(
                "Update available: v{} → v{}",
                current_version, latest_version
            );
            println!("Run 'gf upgrade' to install.");
        } else {
            println!("Updating gf v{} → v{}", current_version, latest_version);
            println!();

            run_install_script().context("failed to run install script")?;

            println!();
            println!("Update complete!");
        }
    } else {
        println!();
        println!("Already up to date.");
    }

    Ok(())
}

/// Fetch the latest version tag from GitHub releases API.
fn fetch_latest_version() -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let body = ureq::get(&url)
        .header("User-Agent", "gf-updater")
        .call()
        .context("failed to connect to GitHub API")?
        .body_mut()
        .read_to_string()
        .context("failed to read GitHub API response")?;

    let response: serde_json::Value =
        serde_json::from_str(&body).context("failed to parse GitHub API response")?;

    let tag = response["tag_name"]
        .as_str()
        .context("no tag_name in release")?;

    // Strip leading 'v' if present
    Ok(tag.trim_start_matches('v').to_string())
}

/// Compare versions and return true if `latest` is newer than `current`.
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ))
        } else {
            None
        }
    };

    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => latest != current,
    }
}

/// Run the install script to download and install the latest version.
fn run_install_script() -> Result<()> {
    use std::process::Command;

    let status = Command::new("bash")
        .arg("-c")
        .arg(format!("curl -LsSf {} | bash", INSTALL_SCRIPT_URL))
        .status()
        .context("failed to execute install script")?;

    if !status.success() {
        anyhow::bail!("install script failed with exit code: {:?}", status.code());
    }

    Ok(())
}

/// Config template for `gf init`.
const CONFIG_TEMPLATE: &str = r#"# gather-files configuration
# Docs: https://github.com/BrianSigafoos/gather-files
version: 1

presets:
  # Example: gather all source code
  # src:
  #   include:
  #     - "**/*.rs"
  #     - "**/*.ts"
  #     - "**/*.py"
  #   exclude:
  #     - "**/node_modules/**"
  #     - "**/target/**"

  # Example: gather documentation
  # docs:
  #   base: docs
  #   include:
  #     - "**/*.md"

  # Example: gather config files
  # config:
  #   include:
  #     - "*.toml"
  #     - "*.yaml"
  #     - "*.json"
  #   exclude:
  #     - "package-lock.json"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_version_detects_major_upgrade() {
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("1.0.1", "1.0.0"));
    }

    #[test]
    fn is_newer_version_returns_false_for_same_version() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.1.22", "0.1.22"));
    }

    #[test]
    fn is_newer_version_returns_false_for_older_version() {
        assert!(!is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.0.0", "1.1.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_version_handles_double_digit_versions() {
        assert!(is_newer_version("0.1.23", "0.1.22"));
        assert!(is_newer_version("0.2.0", "0.1.99"));
        assert!(is_newer_version("1.0.0", "0.99.99"));
    }
}
