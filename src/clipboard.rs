use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Copy the provided text to the clipboard, trying common platform utilities.
pub fn copy_to_clipboard(contents: &str) -> Result<()> {
    let commands = [
        ("pbcopy", &[][..]),
        ("wl-copy", &[][..]),
        ("xclip", &["-selection", "clipboard"]),
        ("clip", &[][..]),
    ];

    for (cmd, args) in commands {
        if try_copy(cmd, args, contents).unwrap_or(false) {
            return Ok(());
        }
    }

    anyhow::bail!("failed to copy to clipboard (no supported clipboard command found)");
}

fn try_copy(cmd: &str, args: &[&str], contents: &str) -> Result<bool> {
    let mut child = match Command::new(cmd).args(args).stdin(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e).with_context(|| format!("failed to run '{cmd}'")),
    };

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(contents.as_bytes())
            .with_context(|| format!("failed to write to '{cmd}' stdin"))?;
    }

    let status = child.wait()?;
    Ok(status.success())
}
