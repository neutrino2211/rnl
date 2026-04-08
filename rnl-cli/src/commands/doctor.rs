//! Doctor command - check build environment

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

pub fn run() -> Result<()> {
    println!("{}", "RNL Doctor - Checking build environment".cyan().bold());
    println!();

    let mut all_ok = true;

    // Check Rust/Cargo
    all_ok &= check_command("cargo", &["--version"], "Rust toolchain");

    // Check Node.js
    all_ok &= check_command("node", &["--version"], "Node.js");

    // Check npm
    all_ok &= check_command("npm", &["--version"], "npm");

    // Check esbuild
    all_ok &= check_command("npx", &["esbuild", "--version"], "esbuild");

    println!();
    println!("{}", "Platform-specific tools:".cyan());
    println!();

    // Linux tools
    println!("  {}", "Linux:".yellow());
    check_command_optional("g++", &["--version"], "  GCC/G++");
    check_command_optional("pkg-config", &["--version"], "  pkg-config");
    check_pkg_config("gtk4", "  GTK4");
    check_pkg_config("libadwaita-1", "  libadwaita");

    // macOS tools
    println!();
    println!("  {}", "macOS:".yellow());
    check_command_optional("swiftc", &["--version"], "  Swift compiler");
    check_command_optional("xcodebuild", &["-version"], "  Xcode");

    // Windows tools
    println!();
    println!("  {}", "Windows:".yellow());
    check_command_optional("dotnet", &["--version"], "  .NET SDK");

    println!();

    if all_ok {
        println!("{}", "✓ Core dependencies are satisfied!".green().bold());
    } else {
        println!(
            "{}",
            "✗ Some dependencies are missing. See above for details.".red().bold()
        );
    }

    println!();
    println!(
        "{}",
        "Note: Platform-specific tools are only needed for their respective platforms.".white()
    );

    Ok(())
}

fn check_command(cmd: &str, args: &[&str], name: &str) -> bool {
    match Command::new(cmd).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            println!("  {} {} {}", "✓".green(), name, version.white());
            true
        }
        _ => {
            println!("  {} {} {}", "✗".red(), name, "not found".red());
            false
        }
    }
}

fn check_command_optional(cmd: &str, args: &[&str], name: &str) {
    match Command::new(cmd).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            // Truncate long version strings
            let version_short = if version.len() > 30 {
                format!("{}...", &version[..30])
            } else {
                version
            };
            println!("    {} {} {}", "✓".green(), name.trim(), version_short.white());
        }
        _ => {
            println!("    {} {} {}", "○".yellow(), name.trim(), "not installed".yellow());
        }
    }
}

fn check_pkg_config(package: &str, name: &str) {
    match Command::new("pkg-config")
        .args(&["--modversion", package])
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("    {} {} {}", "✓".green(), name.trim(), version.white());
        }
        _ => {
            println!("    {} {} {}", "○".yellow(), name.trim(), "not installed".yellow());
        }
    }
}
