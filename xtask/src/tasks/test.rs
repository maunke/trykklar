use crate::tool;
use anyhow::Result;

fn nextest() -> Result<()> {
    tool::run(
        "cargo",
        &["nextest", "run", "--workspace"],
        "cargo binstall cargo-nextest",
    )
}

fn test_doc() -> Result<()> {
    tool::run("cargo", &["test", "--doc"], "cargo is required")
}

pub fn test() -> Result<()> {
    nextest()?;
    test_doc()?;
    Ok(())
}
