//! Clean command - remove build artifacts

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run() -> Result<()> {
    let project_dir = std::env::current_dir()?;

    println!("{}", "Cleaning build artifacts...".cyan());

    let dirs_to_clean = [
        "target",
        "dist",
        "core/target",
    ];

    let mut cleaned = false;

    for dir in &dirs_to_clean {
        let path = project_dir.join(dir);
        if path.exists() {
            println!("  {} {}", "removing".yellow(), dir);
            fs::remove_dir_all(&path)?;
            cleaned = true;
        }
    }

    // Also clean node_modules/.cache if it exists
    let node_cache = project_dir.join("node_modules/.cache");
    if node_cache.exists() {
        println!("  {} node_modules/.cache", "removing".yellow());
        fs::remove_dir_all(&node_cache)?;
        cleaned = true;
    }

    if cleaned {
        println!();
        println!("{}", "✓ Clean complete!".green().bold());
    } else {
        println!("{}", "Nothing to clean.".white());
    }

    Ok(())
}
