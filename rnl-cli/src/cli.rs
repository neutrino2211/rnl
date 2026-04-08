//! CLI argument parsing with clap

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rnl")]
#[command(author = "Mainasara Tsowa")]
#[command(version)]
#[command(about = "RNL - React Native Libre build tool", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new RNL project
    Init(InitOpts),

    /// Build the project for specified platform(s)
    Build(BuildOpts),

    /// Build and run the app
    Run(RunOpts),

    /// Add an element or platform to the project
    Add(AddOpts),

    /// Check build environment and dependencies
    Doctor,

    /// Remove build artifacts
    Clean,
}

#[derive(Parser)]
pub struct InitOpts {
    /// Name of the project to create
    pub name: String,

    /// Platforms to enable (comma-separated: linux,macos,windows)
    #[arg(short, long, default_value = "linux")]
    pub platforms: String,

    /// Directory to create the project in (defaults to ./<name>)
    #[arg(short, long)]
    pub dir: Option<String>,
}

#[derive(Parser)]
pub struct BuildOpts {
    /// Target platform (linux, macos, windows, or all)
    #[arg(short, long, default_value = "linux")]
    pub platform: String,

    /// Build in release mode
    #[arg(short, long)]
    pub release: bool,

    /// Only bundle JS, skip native compilation
    #[arg(long)]
    pub bundle_only: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Parser)]
pub struct RunOpts {
    /// Target platform (linux, macos, windows)
    #[arg(short, long, default_value = "linux")]
    pub platform: String,

    /// Build in release mode
    #[arg(short, long)]
    pub release: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum AddTarget {
    /// Add a new custom element
    Element { name: String },

    /// Add support for a new platform
    Platform { name: String },
}

#[derive(Parser)]
pub struct AddOpts {
    #[command(subcommand)]
    pub target: AddTarget,
}
