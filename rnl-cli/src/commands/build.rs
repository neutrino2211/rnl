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

    // Step 3: Build and link platform code
    for platform in &target_platforms {
        println!();
        println!("{} Building {} platform...", "→".cyan(), platform.yellow());
        build_platform(&project_dir, platform, opts.release, opts.verbose, &config)?;
    }

    println!();
    println!("{}", "✓ Build complete!".green().bold());

    // Print output location
    let mode = if opts.release { "release" } else { "debug" };
    for platform in &target_platforms {
        let binary_name = &config.project.name;
        println!(
            "  {} target/{}/{}",
            "→".green(),
            platform,
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

    // Check if there's a Cargo.toml
    if !core_dir.join("Cargo.toml").exists() {
        println!("  {} Core Cargo.toml not found, skipping", "note:".yellow());
        return Ok(());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    // Explicitly build only rnl-core to avoid triggering platform builds
    cmd.arg("-p").arg("rnl-core");

    if release {
        cmd.arg("--release");
    }

    // Build from the core directory but it will use the workspace if it exists
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
    config: &Config,
) -> Result<()> {
    match platform {
        "linux" => build_linux(project_dir, release, verbose, config),
        "macos" => build_macos(project_dir, release, verbose, config),
        "windows" => build_windows(project_dir, release, verbose, config),
        _ => bail!("Unknown platform: {}", platform),
    }
}

fn build_linux(project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
    // Check for platform in project first, then in RNL framework directory
    let platform_dir = project_dir.join("platforms/linux");
    
    // If project platform dir doesn't have Cargo.toml, look for RNL framework
    let platform_dir = if platform_dir.join("Cargo.toml").exists() {
        platform_dir
    } else {
        // Try to find the RNL framework's platform directory
        // Look relative to the rnl binary or in common locations
        let rnl_framework_paths = [
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|pp| pp.parent().map(|ppp| ppp.to_path_buf())))
                .flatten()
                .unwrap_or_default()
                .join("platforms/linux"),
            project_dir.join("../platforms/linux"), // If running from within RNL workspace
            std::path::PathBuf::from("/home/openclaw/.openclaw/workspace/projects/rnl/platforms/linux"),
        ];

        let framework_platform = rnl_framework_paths
            .iter()
            .find(|p| p.join("Cargo.toml").exists());

        match framework_platform {
            Some(p) => p.clone(),
            None => {
                println!("  {} Linux platform not found in project or framework", "note:".yellow());
                println!("  {} Platform directories checked:", "note:".yellow());
                println!("    - {}", platform_dir.display());
                for p in &rnl_framework_paths {
                    println!("    - {}", p.display());
                }
                return Ok(());
            }
        }
    };

    // Check if this is a Rust platform (has Cargo.toml) or C++ platform
    let cargo_toml = platform_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        return build_linux_rust_from_dir(&platform_dir, project_dir, release, verbose, config);
    }

    // Fall back to C++ build
    build_linux_cpp(project_dir, release, verbose, config)
}

fn build_linux_rust(project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
    let platform_dir = project_dir.join("platforms/linux");
    build_linux_rust_from_dir(&platform_dir, project_dir, release, verbose, config)
}

fn build_linux_rust_from_dir(platform_dir: &Path, project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
    let mode = if release { "release" } else { "debug" };
    let output_dir = project_dir.join("target/linux");
    std::fs::create_dir_all(&output_dir)?;

    // Check for RNL workspace (contains both core and platform)
    // If platform_dir is part of the RNL workspace, we build from there
    let rnl_workspace_root = platform_dir.parent().and_then(|p| p.parent());
    let is_rnl_workspace = rnl_workspace_root
        .map(|root| root.join("core/Cargo.toml").exists() && root.join("Cargo.toml").exists())
        .unwrap_or(false);

    if is_rnl_workspace {
        let workspace_root = rnl_workspace_root.unwrap();
        println!("  Building from RNL workspace at {}", workspace_root.display());
        
        // Build the entire workspace which includes core + platform
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("-p").arg("rnl-linux");
        if release {
            cmd.arg("--release");
        }
        cmd.current_dir(workspace_root);

        if verbose {
            cmd.arg("-v");
        }

        println!("  Compiling Rust platform code...");
        let output = cmd.output().context("Failed to run cargo build for platform")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Check for GTK4 not found error
            if stderr.contains("glib-sys") || stderr.contains("pkg-config") || stderr.contains("gtk4") {
                bail!(
                    "GTK4 development libraries not found.\n\n\
                     Install the required dependencies:\n\
                     - Ubuntu/Debian: sudo apt install libgtk-4-dev pkg-config\n\
                     - Fedora: sudo dnf install gtk4-devel\n\
                     - Arch: sudo pacman -S gtk4\n\n\
                     Note: The code has been written but requires GTK4 to compile.\n\
                     You can test on a machine with GTK4 installed.\n\n\
                     The JS bundle was created successfully at: target/bundle.js"
                );
            }
            
            bail!("Cargo build failed:\n{}", stderr);
        }

        if verbose {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("{}", stdout);
        }

        // Copy the binary to the project's output directory
        let binary_name = &config.project.name;
        let platform_bin = workspace_root.join(format!("target/{}/rnl-linux", mode));
        let output_bin = output_dir.join(binary_name);

        if platform_bin.exists() {
            std::fs::copy(&platform_bin, &output_bin)?;
            println!("  {} {} (Rust/GTK4)", "built".green(), binary_name);
        } else {
            // Try with underscore
            let alt_bin = workspace_root.join(format!("target/{}/rnl_linux", mode));
            if alt_bin.exists() {
                std::fs::copy(&alt_bin, &output_bin)?;
                println!("  {} {} (Rust/GTK4)", "built".green(), binary_name);
            } else {
                println!("  {} Binary built at {}", "note:".yellow(), workspace_root.join(format!("target/{}/", mode)).display());
            }
        }
    } else {
        // Standalone project build (project has its own platform dir)
        // First, build the core library if it exists
        let core_dir = project_dir.join("core");
        if core_dir.exists() && core_dir.join("Cargo.toml").exists() {
            println!("  Building core library...");
            let mut cmd = Command::new("cargo");
            cmd.arg("build");
            if release {
                cmd.arg("--release");
            }
            cmd.current_dir(&core_dir);
            
            let output = cmd.output().context("Failed to build core")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Core build failed:\n{}", stderr);
            }
        }

        // Build the Linux platform
        let core_lib_dir = project_dir.join(format!("core/target/{}", mode));
        
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        if release {
            cmd.arg("--release");
        }
        cmd.current_dir(platform_dir);

        // Set environment variables for linking
        if core_lib_dir.exists() {
            let rustflags = format!("-L {} -l static=rnl", core_lib_dir.display());
            cmd.env("RUSTFLAGS", rustflags);
        }

        if verbose {
            cmd.arg("-v");
        }

        println!("  Compiling Rust platform code...");
        let output = cmd.output().context("Failed to run cargo build for platform")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if stderr.contains("glib-sys") || stderr.contains("pkg-config") || stderr.contains("gtk4") {
                bail!(
                    "GTK4 development libraries not found.\n\n\
                     Install the required dependencies:\n\
                     - Ubuntu/Debian: sudo apt install libgtk-4-dev pkg-config\n\
                     - Fedora: sudo dnf install gtk4-devel\n\
                     - Arch: sudo pacman -S gtk4\n\n\
                     Note: The code has been written but requires GTK4 to compile.\n\
                     You can test on a machine with GTK4 installed.\n\n\
                     The JS bundle was created successfully at: target/bundle.js"
                );
            }
            
            bail!("Cargo build failed:\n{}", stderr);
        }

        if verbose {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("{}", stdout);
        }

        // Copy the binary
        let binary_name = &config.project.name;
        let platform_bin = platform_dir.join(format!("target/{}/rnl-linux", mode));
        let output_bin = output_dir.join(binary_name);

        if platform_bin.exists() {
            std::fs::copy(&platform_bin, &output_bin)?;
            println!("  {} {} (Rust/GTK4)", "built".green(), binary_name);
        }
    }

    Ok(())
}

fn build_linux_cpp(project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
    let platform_dir = project_dir.join("platforms/linux");

    // Use pkg-config to get GTK4 flags
    let pkg_config = Command::new("pkg-config")
        .args(["--cflags", "--libs", "gtk4"])
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
                 Install with: sudo apt install libgtk-4-dev"
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
    let mode = if release { "release" } else { "debug" };
    let obj_dir = project_dir.join(format!("target/linux/{}", mode));
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
        cmd.args(["-c", "-std=c++17"])
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

    // Link
    link_linux_binary(project_dir, &obj_dir, release, config)?;

    Ok(())
}

fn link_linux_binary(project_dir: &Path, obj_dir: &Path, release: bool, config: &Config) -> Result<()> {
    let mode = if release { "release" } else { "debug" };
    let binary_name = &config.project.name;
    let output_dir = project_dir.join("target/linux");
    std::fs::create_dir_all(&output_dir)?;

    // Get GTK4 linker flags
    let pkg_config = Command::new("pkg-config")
        .args(["--libs", "gtk4"])
        .output()?;
    let libs = String::from_utf8_lossy(&pkg_config.stdout);

    // Collect all .o files
    let obj_files: Vec<_> = std::fs::read_dir(obj_dir)?
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
        .arg(output_dir.join(binary_name));

    let output = cmd.output().context("Failed to link")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Linking failed:\n{}", stderr);
    }

    println!("  {} {}", "linked".green(), binary_name);
    Ok(())
}

fn build_macos(project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
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

    let mode = if release { "release" } else { "debug" };
    let obj_dir = project_dir.join(format!("target/macos/{}", mode));
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

    // Link
    let binary_name = &config.project.name;
    let obj_path = obj_dir.join("platform.o");
    let output_dir = project_dir.join("target/macos");
    std::fs::create_dir_all(&output_dir)?;

    let mut cmd = Command::new("swiftc");
    cmd.arg(&obj_path)
        .arg("-framework")
        .arg("AppKit");

    let librnl_path = project_dir.join(format!("core/target/{}/librnl.a", mode));
    if librnl_path.exists() {
        cmd.arg(&librnl_path);
    }

    cmd.arg("-o").arg(output_dir.join(binary_name));

    let output = cmd.output().context("Failed to link")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Linking failed:\n{}", stderr);
    }

    println!("  {} {}", "linked".green(), binary_name);
    Ok(())
}

fn build_windows(project_dir: &Path, release: bool, verbose: bool, config: &Config) -> Result<()> {
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

    let output_dir = project_dir.join("target/windows");
    std::fs::create_dir_all(&output_dir)?;

    // Build with dotnet
    let mut cmd = Command::new("dotnet");
    cmd.arg("build")
        .arg(platform_dir.join("src"));

    if release {
        cmd.arg("-c").arg("Release");
    }

    cmd.arg("-o").arg(&output_dir);

    let output = cmd.output().context("Failed to run dotnet build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dotnet build failed:\n{}", stderr);
    }

    println!("  {} Windows platform", "built".green());
    Ok(())
}
