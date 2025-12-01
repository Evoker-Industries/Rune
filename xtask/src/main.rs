//! Rune xtask - Build automation tasks
//!
//! This crate provides build automation for the Rune container runtime project.
//!
//! ## Usage
//!
//! ```bash
//! # Build everything
//! cargo xtask build
//!
//! # Build WASM modules
//! cargo xtask build-wasm
//!
//! # Build Debian package
//! cargo xtask build-deb
//!
//! # Run all tests
//! cargo xtask test
//!
//! # Run lints
//! cargo xtask lint
//!
//! # Format code
//! cargo xtask fmt
//!
//! # Clean build artifacts
//! cargo xtask clean
//!
//! # Install locally
//! cargo xtask install
//!
//! # Build release
//! cargo xtask release
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for Rune container runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build all components (native + WASM)
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build all native binaries (rune, rune-tui, runefile-lsp)
    BuildNative {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build WASM modules (LSP, Builder, and Rune)
    BuildWasm {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build WASM LSP module only
    BuildWasmLsp {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build WASM Builder module only
    BuildWasmBuilder {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build WASM Rune client module only
    BuildWasmRune {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build Debian package
    BuildDeb,
    /// Build only the main rune binary
    BuildRune {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build only the TUI binary
    BuildTui {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Build only the LSP binary (native)
    BuildLsp {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Run all tests
    Test {
        /// Run tests in release mode
        #[arg(long)]
        release: bool,
    },
    /// Test WASM LSP module
    TestWasmLsp,
    /// Test WASM Builder module
    TestWasmBuilder,
    /// Test WASM Rune client module
    TestWasmRune,
    /// Run lints (clippy and rustfmt check)
    Lint,
    /// Format code
    Fmt {
        /// Check formatting without making changes
        #[arg(long)]
        check: bool,
    },
    /// Clean build artifacts
    Clean,
    /// Install binaries locally
    Install,
    /// Build release artifacts
    Release,
    /// Generate documentation
    Doc {
        /// Open documentation in browser
        #[arg(long)]
        open: bool,
    },
    /// Run CI checks (lint, test, build)
    Ci,
    /// Package VSCode extension
    PackageVscode,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;
    
    // Change to project root
    let project_root = project_root()?;
    sh.change_dir(&project_root);

    match cli.command {
        Commands::Build { release } => build_all(&sh, release)?,
        Commands::BuildNative { release } => build_native(&sh, release)?,
        Commands::BuildWasm { release } => build_wasm(&sh, release)?,
        Commands::BuildWasmLsp { release } => build_wasm_lsp(&sh, release)?,
        Commands::BuildWasmBuilder { release } => build_wasm_builder(&sh, release)?,
        Commands::BuildWasmRune { release } => build_wasm_rune(&sh, release)?,
        Commands::BuildDeb => build_deb(&sh)?,
        Commands::BuildRune { release } => build_rune(&sh, release)?,
        Commands::BuildTui { release } => build_tui(&sh, release)?,
        Commands::BuildLsp { release } => build_lsp(&sh, release)?,
        Commands::Test { release } => test(&sh, release)?,
        Commands::TestWasmLsp => test_wasm_lsp(&sh)?,
        Commands::TestWasmBuilder => test_wasm_builder(&sh)?,
        Commands::TestWasmRune => test_wasm_rune(&sh)?,
        Commands::Lint => lint(&sh)?,
        Commands::Fmt { check } => fmt(&sh, check)?,
        Commands::Clean => clean(&sh)?,
        Commands::Install => install(&sh)?,
        Commands::Release => release(&sh)?,
        Commands::Doc { open } => doc(&sh, open)?,
        Commands::Ci => ci(&sh)?,
        Commands::PackageVscode => package_vscode(&sh)?,
    }

    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let output = std::process::Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .context("Failed to run cargo locate-project")?;
    
    let path = String::from_utf8(output.stdout)?;
    let manifest = PathBuf::from(path.trim());
    
    manifest
        .parent()
        .map(|p| p.to_path_buf())
        .context("Failed to find project root")
}

#[allow(dead_code)]
fn build(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building Rune...");
    
    if release {
        cmd!(sh, "cargo build --release").run()?;
    } else {
        cmd!(sh, "cargo build").run()?;
    }
    
    println!("✅ Build complete!");
    Ok(())
}

fn build_all(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building all components (native + WASM)...");
    
    // Build native binaries
    build_native(sh, release)?;
    
    // Build WASM modules
    build_wasm(sh, release)?;
    
    println!("✅ All builds complete!");
    Ok(())
}

fn build_native(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building native binaries...");
    
    if release {
        cmd!(sh, "cargo build --release --bin rune --bin rune-tui --bin runefile-lsp").run()?;
    } else {
        cmd!(sh, "cargo build --bin rune --bin rune-tui --bin runefile-lsp").run()?;
    }
    
    println!("✅ Native build complete!");
    Ok(())
}

fn build_wasm(sh: &Shell, release: bool) -> Result<()> {
    println!("🌐 Building WASM modules...");
    
    build_wasm_lsp(sh, release)?;
    build_wasm_builder(sh, release)?;
    build_wasm_rune(sh, release)?;
    
    println!("✅ WASM build complete!");
    Ok(())
}

fn build_wasm_lsp(sh: &Shell, release: bool) -> Result<()> {
    println!("🌐 Building WASM LSP module...");
    
    // Check if wasm-pack is installed
    ensure_wasm_pack(sh)?;
    
    let target = if release { "--release" } else { "--dev" };
    
    sh.change_dir("lsp-wasm");
    cmd!(sh, "wasm-pack build --target web {target}").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM LSP build complete!");
    Ok(())
}

fn build_wasm_builder(sh: &Shell, release: bool) -> Result<()> {
    println!("🌐 Building WASM Builder module...");
    
    // Check if wasm-pack is installed
    ensure_wasm_pack(sh)?;
    
    let target = if release { "--release" } else { "--dev" };
    
    sh.change_dir("builder-wasm");
    cmd!(sh, "wasm-pack build --target web {target}").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM Builder build complete!");
    Ok(())
}

fn build_wasm_rune(sh: &Shell, release: bool) -> Result<()> {
    println!("🌐 Building WASM Rune client module...");
    
    // Check if wasm-pack is installed
    ensure_wasm_pack(sh)?;
    
    let target = if release { "--release" } else { "--dev" };
    
    sh.change_dir("rune-wasm");
    cmd!(sh, "wasm-pack build --target web {target}").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM Rune client build complete!");
    Ok(())
}

fn ensure_wasm_pack(sh: &Shell) -> Result<()> {
    if cmd!(sh, "wasm-pack --version").run().is_err() {
        println!("📦 Installing wasm-pack...");
        cmd!(sh, "cargo install wasm-pack").run()?;
    }
    Ok(())
}

fn build_rune(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building rune binary...");
    
    if release {
        cmd!(sh, "cargo build --release --bin rune").run()?;
    } else {
        cmd!(sh, "cargo build --bin rune").run()?;
    }
    
    println!("✅ rune build complete!");
    Ok(())
}

fn build_tui(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building rune-tui binary...");
    
    if release {
        cmd!(sh, "cargo build --release --bin rune-tui").run()?;
    } else {
        cmd!(sh, "cargo build --bin rune-tui").run()?;
    }
    
    println!("✅ rune-tui build complete!");
    Ok(())
}

fn build_lsp(sh: &Shell, release: bool) -> Result<()> {
    println!("🔨 Building runefile-lsp binary...");
    
    if release {
        cmd!(sh, "cargo build --release --bin runefile-lsp").run()?;
    } else {
        cmd!(sh, "cargo build --bin runefile-lsp").run()?;
    }
    
    println!("✅ runefile-lsp build complete!");
    Ok(())
}

fn build_deb(sh: &Shell) -> Result<()> {
    println!("📦 Building Debian package...");
    
    // First build in release mode
    cmd!(sh, "cargo build --release").run()?;
    
    // Check for dpkg-buildpackage
    if cmd!(sh, "dpkg-buildpackage --version").run().is_err() {
        anyhow::bail!("dpkg-buildpackage not found. Install with: apt install dpkg-dev");
    }
    
    // Create debian build directory
    let build_dir = Path::new("target/debian");
    if !build_dir.exists() {
        std::fs::create_dir_all(build_dir)?;
    }
    
    // Copy debian files
    cmd!(sh, "cp -r packaging/debian target/").run()?;
    
    // Build package
    sh.change_dir("target");
    cmd!(sh, "dpkg-buildpackage -us -uc -b").run()?;
    sh.change_dir("..");
    
    println!("✅ Debian package built in target/");
    Ok(())
}

fn test(sh: &Shell, release: bool) -> Result<()> {
    println!("🧪 Running tests...");
    
    if release {
        cmd!(sh, "cargo test --release").run()?;
    } else {
        cmd!(sh, "cargo test").run()?;
    }
    
    // Test WASM modules
    println!("  Testing WASM modules...");
    test_wasm_lsp(sh)?;
    test_wasm_builder(sh)?;
    test_wasm_rune(sh)?;
    
    println!("✅ All tests passed!");
    Ok(())
}

fn test_wasm_lsp(sh: &Shell) -> Result<()> {
    println!("🧪 Testing WASM LSP module...");
    
    sh.change_dir("lsp-wasm");
    cmd!(sh, "cargo test").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM LSP tests passed!");
    Ok(())
}

fn test_wasm_builder(sh: &Shell) -> Result<()> {
    println!("🧪 Testing WASM Builder module...");
    
    sh.change_dir("builder-wasm");
    cmd!(sh, "cargo test").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM Builder tests passed!");
    Ok(())
}

fn test_wasm_rune(sh: &Shell) -> Result<()> {
    println!("🧪 Testing WASM Rune client module...");
    
    sh.change_dir("rune-wasm");
    cmd!(sh, "cargo test").run()?;
    sh.change_dir("..");
    
    println!("✅ WASM Rune client tests passed!");
    Ok(())
}

fn lint(sh: &Shell) -> Result<()> {
    println!("🔍 Running lints...");
    
    // Check formatting
    println!("  Checking formatting...");
    cmd!(sh, "cargo fmt --all -- --check").run()?;
    
    // Run clippy
    println!("  Running clippy...");
    cmd!(sh, "cargo clippy --all-targets --all-features -- -D warnings").run()?;
    
    // Lint WASM modules
    println!("  Linting WASM modules...");
    sh.change_dir("lsp-wasm");
    cmd!(sh, "cargo clippy -- -D warnings").run()?;
    sh.change_dir("..");
    
    sh.change_dir("builder-wasm");
    cmd!(sh, "cargo clippy -- -D warnings").run()?;
    sh.change_dir("..");
    
    sh.change_dir("rune-wasm");
    cmd!(sh, "cargo clippy -- -D warnings").run()?;
    sh.change_dir("..");
    
    println!("✅ All lints passed!");
    Ok(())
}

fn fmt(sh: &Shell, check: bool) -> Result<()> {
    println!("🎨 Formatting code...");
    
    if check {
        cmd!(sh, "cargo fmt --all -- --check").run()?;
    } else {
        cmd!(sh, "cargo fmt --all").run()?;
    }
    
    // Format WASM modules
    sh.change_dir("lsp-wasm");
    if check {
        cmd!(sh, "cargo fmt -- --check").run()?;
    } else {
        cmd!(sh, "cargo fmt").run()?;
    }
    sh.change_dir("..");
    
    sh.change_dir("builder-wasm");
    if check {
        cmd!(sh, "cargo fmt -- --check").run()?;
    } else {
        cmd!(sh, "cargo fmt").run()?;
    }
    sh.change_dir("..");
    
    sh.change_dir("rune-wasm");
    if check {
        cmd!(sh, "cargo fmt -- --check").run()?;
    } else {
        cmd!(sh, "cargo fmt").run()?;
    }
    sh.change_dir("..");
    
    println!("✅ Formatting complete!");
    Ok(())
}

fn clean(sh: &Shell) -> Result<()> {
    println!("🧹 Cleaning build artifacts...");
    
    cmd!(sh, "cargo clean").run()?;
    
    // Clean WASM modules
    sh.change_dir("lsp-wasm");
    cmd!(sh, "cargo clean").run()?;
    let _ = std::fs::remove_dir_all("pkg");
    sh.change_dir("..");
    
    sh.change_dir("builder-wasm");
    cmd!(sh, "cargo clean").run()?;
    let _ = std::fs::remove_dir_all("pkg");
    sh.change_dir("..");
    
    sh.change_dir("rune-wasm");
    cmd!(sh, "cargo clean").run()?;
    let _ = std::fs::remove_dir_all("pkg");
    sh.change_dir("..");
    
    println!("✅ Clean complete!");
    Ok(())
}

fn install(sh: &Shell) -> Result<()> {
    println!("📥 Installing Rune locally...");
    
    cmd!(sh, "cargo install --path .").run()?;
    
    println!("✅ Installation complete!");
    println!("  Installed: rune, rune-tui, runefile-lsp");
    Ok(())
}

fn release(sh: &Shell) -> Result<()> {
    println!("🚀 Building release artifacts...");
    
    // Build main binaries
    println!("  Building release binaries...");
    cmd!(sh, "cargo build --release").run()?;
    
    // Build WASM modules
    println!("  Building WASM modules...");
    build_wasm(sh, true)?;
    
    // Create release directory
    let release_dir = Path::new("target/release-artifacts");
    if release_dir.exists() {
        std::fs::remove_dir_all(release_dir)?;
    }
    std::fs::create_dir_all(release_dir)?;
    
    // Copy binaries
    let binaries = ["rune", "rune-tui", "runefile-lsp"];
    for binary in binaries {
        let src = format!("target/release/{}", binary);
        let dest = format!("target/release-artifacts/{}", binary);
        if Path::new(&src).exists() {
            std::fs::copy(&src, &dest)?;
            println!("  Copied {}", binary);
        }
    }
    
    // Copy WASM artifacts
    std::fs::create_dir_all("target/release-artifacts/wasm")?;
    
    let wasm_dirs = ["lsp-wasm/pkg", "builder-wasm/pkg"];
    for dir in wasm_dirs {
        if Path::new(dir).exists() {
            let dest = format!("target/release-artifacts/wasm/{}", Path::new(dir).parent().unwrap().file_name().unwrap().to_str().unwrap());
            cmd!(sh, "cp -r {dir} {dest}").run()?;
        }
    }
    
    // Copy systemd service files
    std::fs::create_dir_all("target/release-artifacts/systemd")?;
    cmd!(sh, "cp -r packaging/systemd/* target/release-artifacts/systemd/").run()?;
    
    // Copy config examples
    std::fs::create_dir_all("target/release-artifacts/config")?;
    cmd!(sh, "cp -r packaging/config/* target/release-artifacts/config/").run()?;
    
    println!("✅ Release artifacts ready in target/release-artifacts/");
    Ok(())
}

fn doc(sh: &Shell, open: bool) -> Result<()> {
    println!("📚 Generating documentation...");
    
    if open {
        cmd!(sh, "cargo doc --no-deps --open").run()?;
    } else {
        cmd!(sh, "cargo doc --no-deps").run()?;
    }
    
    println!("✅ Documentation generated!");
    Ok(())
}

fn ci(sh: &Shell) -> Result<()> {
    println!("🔄 Running CI checks...");
    
    // Format check
    println!("\n📋 Step 1/4: Format check");
    fmt(sh, true)?;
    
    // Lint
    println!("\n📋 Step 2/4: Lint");
    lint(sh)?;
    
    // Test
    println!("\n📋 Step 3/4: Tests");
    test(sh, false)?;
    
    // Build release
    println!("\n📋 Step 4/4: Release build");
    build(sh, true)?;
    
    println!("\n✅ All CI checks passed!");
    Ok(())
}

fn package_vscode(sh: &Shell) -> Result<()> {
    println!("📦 Packaging VSCode extension...");
    
    // Check if vsce is installed
    if cmd!(sh, "vsce --version").run().is_err() {
        println!("📦 Installing vsce...");
        cmd!(sh, "npm install -g @vscode/vsce").run()?;
    }
    
    sh.change_dir("editors/vscode");
    
    // Install dependencies
    if Path::new("package.json").exists() {
        cmd!(sh, "npm install").run()?;
    }
    
    // Package extension
    cmd!(sh, "vsce package").run()?;
    
    sh.change_dir("../..");
    
    println!("✅ VSCode extension packaged!");
    Ok(())
}
