//! RNL CLI - Build tool for React Native Libre
//!
//! A multi-platform React Native framework where the core is Rust,
//! but platform implementations can be written in native languages.

mod cli;
mod commands;
mod config;
mod templates;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    if let Err(e) = run(args) {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}

fn run(args: cli::Args) -> Result<()> {
    match args.command {
        cli::Command::Init(opts) => commands::init::run(opts),
        cli::Command::Build(opts) => commands::build::run(opts),
        cli::Command::Run(opts) => commands::run::run(opts),
        cli::Command::Add(opts) => commands::add::run(opts),
        cli::Command::Doctor => commands::doctor::run(),
        cli::Command::Clean => commands::clean::run(),
    }
}
