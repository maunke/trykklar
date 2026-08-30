use crate::tool;
use anyhow::Result;

fn rustfmt_check() -> Result<()> {
    tool::run(
        "cargo",
        &["+nightly", "fmt", "--all", "--check"],
        "cargo is required",
    )
}

fn clippy() -> Result<()> {
    tool::run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        "cargo clippy is required",
    )
}

fn dprint() -> Result<()> {
    tool::run("dprint", &["check"], "cargo binstall dprint")
}

fn dependencies() -> Result<()> {
    // https://github.com/embarkstudios/cargo-deny
    let cargo_deny_install_hint = "cargo binstall cargo-deny";
    tool::run(
        "cargo",
        &["deny", "check", "licenses"],
        cargo_deny_install_hint,
    )?;
    tool::run(
        "cargo",
        &["deny", "check", "sources"],
        cargo_deny_install_hint,
    )?;
    tool::run("cargo", &["deny", "check", "bans"], cargo_deny_install_hint)?;
    Ok(())
}

pub fn lint() -> anyhow::Result<()> {
    rustfmt_check()?;
    clippy()?;
    dprint()?;
    dependencies()?;
    Ok(())
}
