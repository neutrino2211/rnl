//! Run command implementation

use crate::cli::RunOpts;
use crate::commands::build;
use crate::config::Config;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

pub fn run(opts: RunOpts) -> Result<()> {
    let project_dir = std::env::current_dir()?;
    
    // Load config
    let config = Config::load(&project_dir)?;

    // Build first
    let build_opts = crate::cli::BuildOpts {
        platform: opts.platform.clone(),
        release: opts.release,
        bundle_only: false,
        verbose: opts.verbose,
    };

    build::run(build_opts)?;

    println!();
    println!("{}", "→ Running application...".cyan());
    println!();

    // Determine binary path
    let mode = if opts.release { "release" } else { "debug" };
    let binary_path = project_dir.join(format!(
        "target/{}/{}/{}",
        opts.platform, mode, config.project.name
    ));

    if !binary_path.exists() {
        bail!(
            "Binary not found at {}. Build may have failed.",
            binary_path.display()
        );
    }

    // Run the binary
    let status = Command::new(&binary_path)
        .current_dir(&project_dir)
        .status()
        .with_context(|| format!("Failed to run {}", binary_path.display()))?;

    if !status.success() {
        if let Some(code) = status.code() {
            bail!("Application exited with code {}", code);
        } else {
            bail!("Application terminated by signal");
        }
    }

    Ok(())
}
