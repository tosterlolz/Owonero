use crate::Cli;
use anyhow::{Result, anyhow};
use clap::CommandFactory;
use clap_complete::{
    generate, generate_to,
    shells::{Bash, Fish, PowerShell, Zsh},
};
use std::fs;
use std::path::PathBuf;

pub fn print_to_stdout(shell: &str) -> Result<()> {
    let mut cmd = Cli::command();
    match shell {
        "bash" => generate(Bash, &mut cmd, "owonero", &mut std::io::stdout()),
        "zsh" => generate(Zsh, &mut cmd, "owonero", &mut std::io::stdout()),
        "fish" => generate(Fish, &mut cmd, "owonero", &mut std::io::stdout()),
        "powershell" | "pwsh" => generate(PowerShell, &mut cmd, "owonero", &mut std::io::stdout()),
        s => return Err(anyhow!("unsupported shell: {}", s)),
    };
    Ok(())
}

/// Install user-level completions and return the exact path written.
pub fn install_user_completion(shell: &str) -> Result<PathBuf> {
    let mut cmd = Cli::command();

    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;

    let out_dir = match shell {
        "bash" => home.join(".local/share/bash-completion/completions"),
        "zsh" => home.join(".local/share/zsh/site-functions"),
        "fish" => home.join(".config/fish/completions"),
        "powershell" | "pwsh" => home.join(".config/powershell/Completions"),
        s => return Err(anyhow!("unsupported shell: {}", s)),
    };

    // Ensure output dir exists
    fs::create_dir_all(&out_dir)?;

    // Use generate_to which returns the actual path written
    let written: PathBuf = match shell {
        "bash" => generate_to(Bash, &mut cmd, "owonero", &out_dir)?,
        "zsh" => generate_to(Zsh, &mut cmd, "owonero", &out_dir)?,
        "fish" => generate_to(Fish, &mut cmd, "owonero", &out_dir)?,
        "powershell" | "pwsh" => generate_to(PowerShell, &mut cmd, "owonero", &out_dir)?,
        _ => unreachable!(),
    };

    Ok(written)
}
