//! Build command implementation

use crate::cli::BuildOpts;
use crate::config::Config;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run(opts: BuildOpts) -> Result<()> {
    let project_dir = std::env::current_dir()?;
    
    // Load config
    let config = Config::load(&project_dir)?;
    
    println!(
        "{} {} ({})",
        "Building".green().bold(),
        config.project.name.cyan(),
        if opts.release { "release" } else { "debug" }
    );

    // Validate platform
    let target_platforms = if opts.platform == "all" {
        config.enabled_platforms()
    } else {
        let platforms: Vec<&str> = opts.platform.split(',').map(|s| s.trim()).collect();
        for p in &platforms {
            if !config.enabled_platforms().contains(p) {
                bail!(
                    "Platform '{}' is not enabled in rnl.toml. Enabled platforms: {:?}",
                    p,
                    config.enabled_platforms()
                );
            }
        }
        platforms
    };

    if opts.verbose {
        println!("  Target platforms: {:?}", target_platforms);
    }

    // Step 1: Bundle JavaScript
    println!();
    println!("{}", "→ Bundling JavaScript...".cyan());
    bundle_js(&project_dir, opts.release, opts.verbose)?;

    if opts.bundle_only {
        println!();
        println!("{}", "✓ Bundle complete (skipping native build)".green().bold());
        return Ok(());
    }

    // Step 2: Build Rust core
    println!();
    println!("{}", "→ Building Rust core...".cyan());
    build_core(&project_dir, opts.release, opts.verbose)?;

    // Step 3: Build platform code
    for platform in &target_platforms {
        println!();
        println!("{} Building {} platform...", "→".cyan(), platform.yellow());
        build_platform(&project_dir, platform, opts.release, opts.verbose)?;
    }

    // Step 4: Link
    println!();
    println!("{}", "→ Linking final binary...".cyan());
    for platform in &target_platforms {
        link_binary(&project_dir, platform, opts.release, opts.verbose)?;
    }

    println!();
    println!("{}", "✓ Build complete!".green().bold());

    // Print output location
    let mode = if opts.release { "release" } else { "debug" };
    for platform in &target_platforms {
        let binary_name = format!("{}", config.project.name);
        println!(
            "  {} target/{}/{}/{}",
            "→".green(),
            platform,
            mode,
            binary_name
        );
    }

    Ok(())
}

fn bundle_js(project_dir: &Path, release: bool, verbose: bool) -> Result<()> {
    let target_dir = project_dir.join("target");
    std::fs::create_dir_all(&target_dir)?;

    let mut args = vec![
        "src/index.tsx".to_string(),
        "--bundle".to_string(),
        "--outfile=target/bundle.js".to_string(),
        "--format=iife".to_string(),
        "--platform=neutral".to_string(),
        "--loader:.tsx=tsx".to_string(),
        "--loader:.ts=ts".to_string(),
    ];

    if release {
        args.push("--minify".to_string());
    } else {
        args.push("--sourcemap".to_string());
    }

    // Try bunx first (for bun users), then npx, then global esbuild
    let runners = [
        ("bunx", vec!["esbuild"]),
        ("npx", vec!["esbuild"]),
        ("esbuild", vec![]),
    ];

    for (runner, prefix_args) in &runners {
        let mut cmd_args: Vec<String> = prefix_args.iter().map(|s| s.to_string()).collect();
        cmd_args.extend(args.clone());

        let result = Command::new(runner)
            .args(&cmd_args)
            .current_dir(project_dir)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    if verbose {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.is_empty() {
                            println!("{}", stdout);
                        }
                    }
                    println!("  {} target/bundle.js", "created".green());
                    return Ok(());
                }
                // If this runner exists but failed, report the error
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains("not found") && !stderr.contains("No such file") {
                    bail!("esbuild failed: {}", stderr);
                }
                // Otherwise try next runner
            }
            Err(_) => continue, // Try next runner
        }
    }

    bail!(
        "esbuild not found. Install it with one of:\n\
         - bun add -d esbuild  (if using bun)\n\
         - npm install         (in project directory)\n\
         - npm install -g esbuild"
    )
}

fn build_core(project_dir: &Path, release: bool, verbose: bool) -> Result<()> {
    let core_dir = project_dir.join("core");
    
    // Check if core directory exists
    if !core_dir.exists() {
        // For now, we'll skip core building if it doesn't exist
        // This will be populated by the framework
        println!("  {} Core not present, skipping (using bundled librnl)", "note:".yellow());
        return Ok(());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if release {
        cmd.arg("--release");
    }

    cmd.current_dir(&core_dir);

    let output = cmd.output().context("Failed to run cargo build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Cargo build failed:\n{}", stderr);
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
    }

    println!("  {} librnl.a", "built".green());
    Ok(())
}

fn build_platform(
    project_dir: &Path,
    platform: &str,
    release: bool,
    verbose: bool,
) -> Result<()> {
    match platform {
        "linux" => build_linux(project_dir, release, verbose),
        "macos" => build_macos(project_dir, release, verbose),
        "windows" => build_windows(project_dir, release, verbose),
        _ => bail!("Unknown platform: {}", platform),
    }
}

fn build_linux(project_dir: &Path, release: bool, verbose: bool) -> Result<()> {
    let platform_dir = project_dir.join("platforms/linux");
    
    if !platform_dir.exists() {
        println!("  {} Linux platform not present, skipping", "note:".yellow());
        return Ok(());
    }

    // Use pkg-config to get GTK4 flags
    let pkg_config = Command::new("pkg-config")
        .args(&["--cflags", "--libs", "gtk4", "libadwaita-1"])
        .output();

    let (cflags, libs) = match pkg_config {
        Ok(output) if output.status.success() => {
            let flags = String::from_utf8_lossy(&output.stdout);
            // Split into cflags (starts with -I or -D) and libs (starts with -l or -L)
            let parts: Vec<&str> = flags.split_whitespace().collect();
            let cflags: Vec<&str> = parts
                .iter()
                .filter(|s| s.starts_with("-I") || s.starts_with("-D"))
                .copied()
                .collect();
            let libs: Vec<&str> = parts
                .iter()
                .filter(|s| s.starts_with("-l") || s.starts_with("-L"))
                .copied()
                .collect();
            (cflags.join(" "), libs.join(" "))
        }
        _ => {
            bail!(
                "GTK4 development libraries not found.\n\
                 Install with: sudo apt install libgtk-4-dev libadwaita-1-dev"
            );
        }
    };

    // Find all .cpp files
    let src_dir = platform_dir.join("src");
    let cpp_files: Vec<_> = walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "cpp").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    if cpp_files.is_empty() {
        println!("  {} No C++ source files found", "note:".yellow());
        return Ok(());
    }

    // Compile each file to .o
    let obj_dir = project_dir.join(format!(
        "target/linux/{}",
        if release { "release" } else { "debug" }
    ));
    std::fs::create_dir_all(&obj_dir)?;

    let optimization = if release { "-O2" } else { "-O0 -g" };

    for cpp_file in &cpp_files {
        let obj_name = cpp_file
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string()
            + ".o";
        let obj_path = obj_dir.join(&obj_name);

        let mut cmd = Command::new("g++");
        cmd.args(&["-c", "-std=c++17"])
            .arg(optimization)
            .args(cflags.split_whitespace())
            .arg("-I")
            .arg(project_dir.join("core/include"))
            .arg(cpp_file)
            .arg("-o")
            .arg(&obj_path);

        let output = cmd.output().context("Failed to run g++")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("g++ compilation failed for {}:\n{}", cpp_file.display(), stderr);
        }

        if verbose {
            println!("  {} {}", "compiled".green(), cpp_file.display());
        }
    }

    println!("  {} {} object files", "compiled".green(), cpp_files.len());
    Ok(())
}

fn build_macos(project_dir: &Path, release: bool, verbose: bool) -> Result<()> {
    let platform_dir = project_dir.join("platforms/macos");
    
    if !platform_dir.exists() {
        println!("  {} macOS platform not present, skipping", "note:".yellow());
        return Ok(());
    }

    // Build with swiftc
    let sources_dir = platform_dir.join("Sources");
    let swift_files: Vec<_> = walkdir::WalkDir::new(&sources_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "swift").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();

    if swift_files.is_empty() {
        println!("  {} No Swift source files found", "note:".yellow());
        return Ok(());
    }

    let obj_dir = project_dir.join(format!(
        "target/macos/{}",
        if release { "release" } else { "debug" }
    ));
    std::fs::create_dir_all(&obj_dir)?;

    let optimization = if release { "-O" } else { "-Onone -g" };

    let mut cmd = Command::new("swiftc");
    cmd.arg("-emit-object")
        .arg(optimization)
        .arg("-import-objc-header")
        .arg(project_dir.join("core/include/rnl.h"))
        .args(&swift_files)
        .arg("-o")
        .arg(obj_dir.join("platform.o"));

    let output = cmd.output().context("Failed to run swiftc")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("swiftc compilation failed:\n{}", stderr);
    }

    println!("  {} {} Swift files", "compiled".green(), swift_files.len());
    Ok(())
}

fn build_windows(project_dir: &Path, release: bool, verbose: bool) -> Result<()> {
    let platform_dir = project_dir.join("platforms/windows");
    
    if !platform_dir.exists() {
        println!("  {} Windows platform not present, skipping", "note:".yellow());
        return Ok(());
    }

    // Check for dotnet or csc
    let dotnet_check = Command::new("dotnet").arg("--version").output();

    if dotnet_check.is_err() || !dotnet_check.unwrap().status.success() {
        bail!(
            ".NET SDK not found.\n\
             Install from: https://dotnet.microsoft.com/download"
        );
    }

    let obj_dir = project_dir.join(format!(
        "target/windows/{}",
        if release { "release" } else { "debug" }
    ));
    std::fs::create_dir_all(&obj_dir)?;

    // Build with dotnet
    let mut cmd = Command::new("dotnet");
    cmd.arg("build")
        .arg(platform_dir.join("src"));

    if release {
        cmd.arg("-c").arg("Release");
    }

    cmd.arg("-o").arg(&obj_dir);

    let output = cmd.output().context("Failed to run dotnet build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dotnet build failed:\n{}", stderr);
    }

    println!("  {} Windows platform", "built".green());
    Ok(())
}

fn link_binary(
    project_dir: &Path,
    platform: &str,
    release: bool,
    verbose: bool,
) -> Result<()> {
    let config = Config::load(project_dir)?;
    let mode = if release { "release" } else { "debug" };
    let obj_dir = project_dir.join(format!("target/{}/{}", platform, mode));
    let binary_name = &config.project.name;

    match platform {
        "linux" => {
            // Get GTK4 linker flags
            let pkg_config = Command::new("pkg-config")
                .args(&["--libs", "gtk4", "libadwaita-1"])
                .output()?;
            let libs = String::from_utf8_lossy(&pkg_config.stdout);

            // Collect all .o files
            let obj_files: Vec<_> = std::fs::read_dir(&obj_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "o").unwrap_or(false))
                .map(|e| e.path())
                .collect();

            if obj_files.is_empty() {
                println!("  {} No object files to link", "note:".yellow());
                return Ok(());
            }

            let mut cmd = Command::new("g++");
            cmd.args(&obj_files);

            // Link against librnl if it exists
            let librnl_path = project_dir.join(format!("core/target/{}/librnl.a", mode));
            if librnl_path.exists() {
                cmd.arg(&librnl_path);
            }

            cmd.args(libs.split_whitespace())
                .arg("-o")
                .arg(obj_dir.join(binary_name));

            let output = cmd.output().context("Failed to link")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Linking failed:\n{}", stderr);
            }

            println!("  {} {}", "linked".green(), binary_name);
        }
        "macos" => {
            // Link with Swift and AppKit
            let obj_path = obj_dir.join("platform.o");
            
            if !obj_path.exists() {
                println!("  {} No object file to link", "note:".yellow());
                return Ok(());
            }

            let mut cmd = Command::new("swiftc");
            cmd.arg(&obj_path)
                .arg("-framework")
                .arg("AppKit");

            let librnl_path = project_dir.join(format!("core/target/{}/librnl.a", mode));
            if librnl_path.exists() {
                cmd.arg(&librnl_path);
            }

            cmd.arg("-o").arg(obj_dir.join(binary_name));

            let output = cmd.output().context("Failed to link")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Linking failed:\n{}", stderr);
            }

            println!("  {} {}", "linked".green(), binary_name);
        }
        "windows" => {
            // Windows linking is handled by dotnet build
            println!("  {} Windows executable", "created".green());
        }
        _ => bail!("Unknown platform: {}", platform),
    }

    Ok(())
}
