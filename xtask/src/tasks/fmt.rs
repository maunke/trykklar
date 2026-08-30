use crate::tool;
use anyhow::Result;

fn rust() -> Result<()> {
    tool::run("cargo", &["+nightly", "fmt", "--all"], "cargo is required")
}

fn dprint() -> Result<()> {
    tool::run("dprint", &["fmt"], "cargo binstall dprint")
}

pub fn fmt() -> Result<()> {
    rust()?;
    dprint()?;
    Ok(())
}
