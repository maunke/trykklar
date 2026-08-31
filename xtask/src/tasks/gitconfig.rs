use crate::tool;
use anyhow::Result;

fn format() -> Result<()> {
    println!("Setup git config format ...");
    let config_commands = [
        ["format.subjectPrefix", "PATCH trykklar"],
        ["format.notes", "true"],
    ];
    for config_args in config_commands {
        let mut args = vec!["config"];
        args.extend_from_slice(&config_args);
        tool::run("git", &args, "git is required")?;
    }
    Ok(())
}

fn sendemail() -> Result<()> {
    println!("Setup git config sendemail ...");
    let config_commands = [
        ["sendemail.to", "~maunke/trykklar-devel@lists.sr.ht"],
        ["sendemail.validate", "true"],
    ];
    for config_args in config_commands {
        let mut args = vec!["config"];
        args.extend_from_slice(&config_args);
        tool::run("git", &args, "git is required")?;
    }
    Ok(())
}

fn hooks() -> Result<()> {
    println!("Setup git hooks ...");
    tool::run("mkdir", &["-p", ".git/hooks"], "")?;
    // sendemail validate hook
    tool::run(
        "ln",
        &[
            "-sf",
            "../../contrib/sendemail-validate",
            ".git/hooks/sendemail-validate",
        ],
        "",
    )?;
    Ok(())
}

pub fn gitconfig() -> Result<()> {
    format()?;
    hooks()?;
    sendemail()?;
    Ok(())
}
