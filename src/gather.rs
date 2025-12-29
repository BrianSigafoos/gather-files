use crate::config::Preset;
use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use globwalk::GlobWalkerBuilder;
use indexmap::IndexSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Directories skipped during recursive walks when gathering paths.
const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules"];

/// Collect files from a directory (or a single file) recursively.
pub fn collect_from_path(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        anyhow::bail!("path '{}' does not exist", path.display());
    }

    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| !is_ignored_dir(e)) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    files.sort_unstable();
    promote_readme(path, &mut files);
    Ok(files)
}

/// Collect files based on preset patterns.
pub fn collect_from_preset(name: &str, preset: &Preset, repo_root: &Path) -> Result<Vec<PathBuf>> {
    let base = resolve_base(preset, repo_root);
    let exclude = build_globset(&preset.exclude)?;
    let ignored_patterns = ignored_dir_globs();
    let mut ordered = IndexSet::new();

    for pattern in &preset.include {
        let pattern_matches =
            collect_pattern_matches(name, pattern, &base, &exclude, &ignored_patterns)?;

        if pattern_matches.is_empty() {
            anyhow::bail!("no files matched pattern '{pattern}' in preset '{name}'");
        }

        for path in pattern_matches {
            ordered.insert(path);
        }
    }

    let mut files: Vec<PathBuf> = ordered.into_iter().collect();
    promote_readme(&base, &mut files);
    Ok(files)
}

/// Render file contents in the gather_files format.
pub fn render_files(files: &[PathBuf], root: &Path) -> Result<(String, usize)> {
    let mut output = String::new();
    let mut char_count = 0;

    for path in files {
        let display = display_path(path, root);
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        char_count += append_file_section(&mut output, &display, &contents);
    }

    Ok((output, char_count))
}

fn append_file_section(output: &mut String, display: &str, contents: &str) -> usize {
    const HEADER_PREFIX: &str = "-------\n# ";
    const HEADER_SUFFIX: &str = "\n\n";

    output.reserve(HEADER_PREFIX.len() + display.len() + HEADER_SUFFIX.len() + contents.len() + 2);

    output.push_str(HEADER_PREFIX);
    output.push_str(display);
    output.push_str(HEADER_SUFFIX);
    output.push_str(contents);

    let mut count = HEADER_PREFIX.len();
    count += display.chars().count();
    count += HEADER_SUFFIX.len();
    count += contents.chars().count();

    if !contents.ends_with('\n') {
        output.push('\n');
        count += 1;
    }
    output.push('\n');
    count += 1;

    count
}

fn display_path(path: &Path, root: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(root) {
        if relative.as_os_str().is_empty() {
            return path.display().to_string();
        }
        return relative.display().to_string();
    }

    path.display().to_string()
}

fn resolve_base(preset: &Preset, repo_root: &Path) -> PathBuf {
    match &preset.base {
        Some(base) if base.is_absolute() => base.clone(),
        Some(base) => repo_root.join(base),
        None => repo_root.to_path_buf(),
    }
}

fn ignored_dir_globs() -> Vec<String> {
    IGNORED_DIRS
        .iter()
        .map(|dir| format!("!**/{dir}/"))
        .collect()
}

fn build_preset_patterns<'a>(pattern: &'a str, ignored: &'a [String]) -> Vec<&'a str> {
    let mut patterns = Vec::with_capacity(1 + ignored.len());
    patterns.push(pattern);
    patterns.extend(ignored.iter().map(String::as_str));
    patterns
}

fn collect_pattern_matches(
    preset_name: &str,
    pattern: &str,
    base: &Path,
    exclude: &Option<GlobSet>,
    ignored_patterns: &[String],
) -> Result<Vec<PathBuf>> {
    let patterns = build_preset_patterns(pattern, ignored_patterns);
    let walker = GlobWalkerBuilder::from_patterns(base, &patterns)
        .follow_links(false)
        .build()
        .with_context(|| format!("invalid glob '{pattern}' in preset '{preset_name}'"))?;

    let mut matches = Vec::new();
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.into_path();
        if matches_exclude(exclude, base, &path) {
            continue;
        }

        matches.push(path);
    }

    matches.sort_unstable();
    Ok(matches)
}

fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .with_context(|| format!("invalid exclude glob pattern '{pattern}'"))?;
        builder.add(glob);
    }

    let set = builder.build()?;
    Ok(Some(set))
}

fn matches_exclude(set: &Option<GlobSet>, base: &Path, path: &Path) -> bool {
    match set {
        Some(set) => {
            let candidate = path.strip_prefix(base).unwrap_or(path);
            set.is_match(candidate)
        }
        None => false,
    }
}

fn promote_readme(base: &Path, files: &mut Vec<PathBuf>) {
    if files.len() <= 1 {
        return;
    }

    if let Some(idx) = find_preferred_readme(base, files)
        && idx != 0
    {
        let readme = files.remove(idx);
        files.insert(0, readme);
    }
}

fn find_preferred_readme(base: &Path, files: &[PathBuf]) -> Option<usize> {
    let mut fallback = None;
    for (idx, path) in files.iter().enumerate() {
        if !is_readme(path) {
            continue;
        }

        if is_direct_child(base, path) {
            return Some(idx);
        }

        if fallback.is_none() {
            fallback = Some(idx);
        }
    }

    fallback
}

fn is_direct_child(base: &Path, path: &Path) -> bool {
    match path.strip_prefix(base) {
        Ok(relative) => relative.components().count() == 1,
        Err(_) => false,
    }
}

fn is_readme(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase().starts_with("readme"))
        .unwrap_or(false)
}

fn is_ignored_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    let name = entry.file_name();
    let name = match name.to_str() {
        Some(name) => name,
        None => return false,
    };

    IGNORED_DIRS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigFile;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn promotes_readme_in_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        write_file(path.join("README.md"), "# hi");
        write_file(path.join("b.txt"), "b");
        write_file(path.join("a.txt"), "a");

        let files = collect_from_path(path).unwrap();
        assert_eq!(
            files
                .iter()
                .map(|p| p.file_name().unwrap().to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["README.md", "a.txt", "b.txt"]
        );
    }

    #[test]
    fn collects_files_from_preset() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        write_file(base.join("README.md"), "root");
        write_file(base.join("src/lib.rs"), "lib");
        write_file(base.join("src/main.rs"), "main");
        write_file(base.join("src/extra.txt"), "extra");

        let config_yaml = r#"
version: 1
presets:
  rust:
    base: .
    include:
      - "src/**/*.rs"
    exclude:
      - "src/lib.rs"
"#;
        let config_path = base.join(".gather-files.yaml");
        fs::write(&config_path, config_yaml).unwrap();
        let config = ConfigFile::load(&config_path).unwrap().unwrap();
        let preset = config.preset("rust").unwrap();
        let files = collect_from_preset("rust", preset, base).unwrap();
        assert_eq!(
            files
                .iter()
                .map(|p| p.strip_prefix(base).unwrap().display().to_string())
                .collect::<Vec<_>>(),
            vec!["src/main.rs"]
        );
    }

    #[test]
    fn render_includes_headers() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let file = base.join("README.md");
        write_file(file.clone(), "Hello world\n");

        let output = render_files(&[file], base).unwrap();
        assert!(output.0.contains("# README.md"));
        assert!(output.0.contains("Hello world"));
        assert_eq!(output.1, output.0.chars().count());
    }

    #[test]
    fn preset_skips_ignored_directories() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        write_file(base.join("README.md"), "root");
        write_file(base.join("src/main.rs"), "main");
        write_file(base.join("target/ignored.rs"), "ignore");
        write_file(base.join("node_modules/pkg/index.js"), "ignore");

        let config_yaml = r#"
version: 1
presets:
  everything:
    base: .
    include:
      - "**/*"
"#;
        let config_path = base.join(".gather-files.yaml");
        fs::write(&config_path, config_yaml).unwrap();
        let config = ConfigFile::load(&config_path).unwrap().unwrap();
        let preset = config.preset("everything").unwrap();
        let files = collect_from_preset("everything", preset, base).unwrap();
        let paths = files
            .iter()
            .map(|p| p.strip_prefix(base).unwrap().display().to_string())
            .collect::<Vec<_>>();

        assert!(paths.contains(&"README.md".to_string()));
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(!paths.iter().any(|path| path.starts_with("target/")));
        assert!(!paths.iter().any(|path| path.starts_with("node_modules/")));
    }

    fn write_file(path: PathBuf, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        writeln!(file, "{contents}").unwrap();
    }
}
