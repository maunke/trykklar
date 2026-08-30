use anyhow::{Result, anyhow};
use std::process::Command;

pub fn run(program: &str, args: &[&str], install_hint: &str) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(crate::project_root())
        .status()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                anyhow!("{program} not found — install with `{install_hint}`")
            }
            _ => e.into(),
        })?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
