use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed representation of `.gather-files.yaml`.
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub version: u32,
    #[serde(default)]
    pub presets: IndexMap<String, Preset>,
}

/// A named preset describing which files to gather.
#[derive(Debug, Deserialize, Clone)]
pub struct Preset {
    /// Glob patterns to include (relative to `base` if provided).
    pub include: Vec<String>,
    /// Glob patterns to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Optional base directory to apply includes/excludes against.
    #[serde(default)]
    pub base: Option<PathBuf>,
}

impl ConfigFile {
    /// Load configuration from disk if the file exists.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let config: ConfigFile = serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;
        config.validate()?;
        Ok(Some(config))
    }

    fn validate(&self) -> Result<()> {
        if self.version != 1 {
            anyhow::bail!("unsupported config version {} (expected 1)", self.version);
        }

        for (name, preset) in &self.presets {
            if preset.include.is_empty() {
                anyhow::bail!("preset '{name}' must define at least one include pattern");
            }
        }

        Ok(())
    }

    /// Fetch a preset by name.
    pub fn preset(&self, name: &str) -> Option<&Preset> {
        self.presets.get(name)
    }
}
