use std::path::{Path, PathBuf};
use tracing::{info, warn};
use color_eyre::Result;
use yansi::{Color, Paint};
use crate::args::DarwinCleanArgs;
use nh_core::command::ElevationStrategy;
use nh_core::ui::*;
use std::collections::HashSet;
use std::sync::LazyLock;

static IGNORE_DIRS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert(".git");
    s.insert(".svn");
    s.insert(".hg");
    s.insert(".Trash");
    s.insert(".Trashes");
    s.insert("Library");
    s.insert("Applications");
    s.insert("System");
    s.insert("Volumes");
    s.insert("dev");
    s.insert("bin");
    s.insert("sbin");
    s.insert("usr");
    s.insert("etc");
    s.insert("var");
    s.insert("private");
    s.insert("cores");
    s.insert(".nvm");
    s.insert(".rustup");
    s.insert(".pyenv");
    s.insert(".vscode");
    s.insert(".idea");
    s
});

static TARGET_DIRS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("node_modules");
    s.insert("target");
    s.insert("build");
    s.insert("dist");
    s.insert("venv");
    s.insert(".venv");
    s.insert("__pycache__");
    s.insert(".next");
    s.insert(".turbo");
    s
});

pub fn run_clean_darwin(args: &DarwinCleanArgs, _elevate: ElevationStrategy) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(eyre!("nh clean darwin is only supported on macOS"));
    }

    #[cfg(target_os = "macos")]
    {
        println!();
        println!(
            "{} {} {}",
            Paint::new(ICON_ARROW).fg(PURPLE).bold(),
            Paint::new("nh clean darwin").bold(),
            if args.dry {
                Paint::new("(dry-run)").fg(YELLOW).to_string()
            } else {
                "".to_string()
            }
        );
        println!(
            "  {}",
            Paint::new("Deep cleaning macOS (experimental, ported from Mole)").dim()
        );
        
        let mut total_freed: u64 = 0;
        let mut total_potential: u64 = 0;

        for category in &args.categories {
            let (freed, potential) = match category.as_str() {
                "system" => (clean_system(args)?, 0),
                "user" => (clean_user(args)?, 0),
                "dev" => (clean_dev(args)?, 0),
                "apps" => (clean_apps(args)?, 0),
                "browsers" => (clean_browsers(args)?, 0),
                "optimize" => { run_optimize(args)?; (0, 0) },
                "purge" => (0, clean_purge(args)?),
                "nix" => { clean_nix(args)?; (0, 0) },
                _ => { warn!("Unknown category: {}", category); (0, 0) },
            };
            total_freed += freed;
            total_potential += potential;
        }

        println!();
        if args.dry {
            println!(
                "{} {}",
                Paint::new(ICON_DRY_RUN).fg(YELLOW).bold(),
                Paint::new(format!("Potential space to free: {}", bytes_to_human(total_freed + total_potential))).bold()
            );
        } else {
            println!(
                "{} {}",
                Paint::new(ICON_SUCCESS).fg(GREEN).bold(),
                Paint::new(format!("Cleanup complete! Total space freed: {}", bytes_to_human(total_freed))).bold()
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn clean_system(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("System Cleanup").bold());
    let mut freed = 0;
    
    // System Caches (Clean contents)
    freed += safe_remove_contents("/Library/Caches", args, true)?;
    
    // System Logs (Clean contents)
    freed += safe_remove_contents("/private/var/log", args, true)?;
    
    // Diagnostic Reports (Clean contents)
    freed += safe_remove_contents("/Library/Logs/DiagnosticReports", args, true)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_user(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("User Cleanup").bold());
    let mut freed = 0;
    let home = std::env::var("HOME")?;
    
    // User Caches (Clean contents)
    freed += safe_remove_contents(&format!("{}/Library/Caches", home), args, false)?;
    
    // User Logs (Clean contents)
    freed += safe_remove_contents(&format!("{}/Library/Logs", home), args, false)?;
    
    // Trash (Clean contents)
    freed += safe_remove_contents(&format!("{}/.Trash", home), args, false)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_dev(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("Developer Tools Cleanup").bold());
    let mut freed = 0;
    let home = std::env::var("HOME")?;

    // Node/NPM
    freed += safe_remove(&format!("{}/.npm/_cacache", home), args, false)?;
    
    // Rust/Cargo
    freed += safe_remove(&format!("{}/.cargo/registry/cache", home), args, false)?;
    freed += safe_remove(&format!("{}/.cargo/git/db", home), args, false)?;
    
    // Go
    freed += safe_remove(&format!("{}/Library/Caches/go-build", home), args, false)?;
    
    // Xcode
    freed += safe_remove(&format!("{}/Library/Developer/Xcode/DerivedData", home), args, false)?;
    freed += safe_remove(&format!("{}/Library/Developer/Xcode/Archives", home), args, false)?;

    // Containers (Docker & Colima)
    freed += clean_containers(args)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_containers(args: &DarwinCleanArgs) -> Result<u64> {
    let mut freed = 0;
    let home = std::env::var("HOME")?;

    // Docker cleanup
    let docker_exists = std::process::Command::new("docker")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

    if docker_exists {
        if args.dry {
            println!("  {} Would run docker system prune -af --volumes", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        } else {
            // Check if docker is running to avoid hanging
            let status = std::process::Command::new("docker")
                .arg("info")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            if status.is_ok() && status.unwrap().success() {
                info!("Pruning Docker system...");
                let _ = std::process::Command::new("docker")
                    .args(["system", "prune", "-af", "--volumes"])
                    .status();
                println!("  {} Docker system pruned", Paint::new("SUCCESS:").fg(Color::Green));
            }
        }
    }

    // Colima specific cleanup (logs and archives)
    freed += safe_remove(&format!("{}/.colima/_archive", home), args, false)?;
    freed += safe_remove(&format!("{}/.colima/default/cache", home), args, false)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_apps(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("Application Cleanup").bold());
    let mut freed = 0;
    let home = std::env::var("HOME")?;

    // VS Code
    freed += safe_remove(&format!("{}/Library/Application Support/Code/Cache", home), args, false)?;
    freed += safe_remove(&format!("{}/Library/Application Support/Code/CachedData", home), args, false)?;
    
    // Slack
    freed += safe_remove(&format!("{}/Library/Application Support/Slack/Cache", home), args, false)?;
    
    // Discord
    freed += safe_remove(&format!("{}/Library/Application Support/discord/Cache", home), args, false)?;
    
    // Spotify
    freed += safe_remove(&format!("{}/Library/Caches/com.spotify.client", home), args, false)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_browsers(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("Browser Cleanup").bold());
    let mut freed = 0;
    let home = std::env::var("HOME")?;

    // Safari
    freed += safe_remove(&format!("{}/Library/Caches/com.apple.Safari", home), args, false)?;
    
    // Chrome
    freed += safe_remove(&format!("{}/Library/Caches/Google/Chrome", home), args, false)?;

    Ok(freed)
}

#[cfg(target_os = "macos")]
fn clean_purge(args: &DarwinCleanArgs) -> Result<u64> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("Project Purge (Build Artifacts)").bold());
    let mut total_freed = 0;
    let home = std::env::var("HOME")?;
    
    let search_roots = vec![
        PathBuf::from(format!("{}/Projects", home)),
        PathBuf::from(format!("{}/GitHub", home)),
        PathBuf::from(format!("{}/dev", home)),
        PathBuf::from(format!("{}/workspace", home)),
        PathBuf::from(format!("{}/personal", home)),
        PathBuf::from(format!("{}/work", home)),
        PathBuf::from(format!("{}/code", home)),
        PathBuf::from(format!("{}/src", home)),
        PathBuf::from(format!("{}/Documents/Projects", home)),
    ];

    for root in search_roots {
        if root.exists() {
            total_freed += scan_and_purge(&root, args)?;
        }
    }

    Ok(total_freed)
}

#[cfg(target_os = "macos")]
fn scan_and_purge(path: &Path, args: &DarwinCleanArgs) -> Result<u64> {
    let mut freed = 0;
    
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if !ft.is_dir() || ft.is_symlink() {
                continue;
            }

            let entry_path = entry.path();
            let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
            
            if TARGET_DIRS.contains(&*name) {
                freed += safe_remove(&entry_path.to_string_lossy(), args, false)?;
            } else if !IGNORE_DIRS.contains(&*name) {
                // Recursive scan if not ignored and not a target
                freed += scan_and_purge(&entry_path, args)?;
            }
        }
    }
    
    Ok(freed)
}

#[cfg(target_os = "macos")]
fn run_optimize(args: &DarwinCleanArgs) -> Result<()> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("System Optimization").bold());
    
    if args.dry {
        println!("  {} Would flush DNS cache", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        println!("  {} Would rebuild launch services", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        println!("  {} Would refresh Finder and Dock", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        return Ok(());
    }

    // Flush DNS Cache
    info!("Flushing DNS cache...");
    let _ = std::process::Command::new("sudo")
        .args(["dscacheutil", "-flushcache"])
        .status();
    let _ = std::process::Command::new("sudo")
        .args(["killall", "-HUP", "mDNSResponder"])
        .status();
    println!("  {} DNS cache flushed", Paint::new(ICON_SUCCESS).fg(GREEN).bold());

    // Rebuild Launch Services
    info!("Rebuilding launch services...");
    let _ = std::process::Command::new("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister")
        .args(["-seed", "-r", "-f", "-domain", "local", "-domain", "system", "-domain", "user"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status();
    println!("  {} Launch services rebuilt", Paint::new(ICON_SUCCESS).fg(GREEN).bold());

    // Refresh Finder and Dock
    info!("Refreshing Finder and Dock...");
    let _ = std::process::Command::new("killall")
        .args(["Finder"])
        .status();
    let _ = std::process::Command::new("killall")
        .args(["Dock"])
        .status();
    println!("  {} Finder and Dock refreshed", Paint::new(ICON_SUCCESS).fg(GREEN).bold());

    Ok(())
}

#[cfg(target_os = "macos")]
fn clean_nix(args: &DarwinCleanArgs) -> Result<()> {
    println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), Paint::new("Nix Cleanup").bold());

    if args.dry {
        println!("  {} Would run nix-collect-garbage -d (Removes all old generations)", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        println!("  {} Would run nix-store --gc (Reclaims store space)", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        println!("  {} Would run nix-store --optimise (Deduplicates store files)", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold());
        return Ok(());
    }

    if args.ask {
        use inquire::Confirm;
        if !Confirm::new("Run Nix garbage collection (requires sudo)?")
            .with_default(false)
            .prompt()? 
        {
            return Ok(());
        }
    }

    info!("Running nix-collect-garbage -d...");
    nh_core::command::Command::new("nix-collect-garbage")
        .arg("-d")
        .elevate(Some(ElevationStrategy::Auto))
        .message("Removing old Nix generations")
        .show_output(false)
        .run()?;

    info!("Running nix-store --gc...");
    nh_core::command::Command::new("nix-store")
        .arg("--gc")
        .elevate(Some(ElevationStrategy::Auto))
        .message("Performing Nix store garbage collection")
        .show_output(false)
        .run()?;

    info!("Running nix-store --optimise... (this may take a while)");
    nh_core::command::Command::new("nix-store")
        .arg("--optimise")
        .elevate(Some(ElevationStrategy::Auto))
        .message("Optimising the Nix store (this may take a while)")
        .show_output(false)
        .run()?;

    println!("  {} Nix cleanup complete", Paint::new(ICON_SUCCESS).fg(GREEN).bold());
    Ok(())
}

#[cfg(target_os = "macos")]
fn safe_remove_contents(path_str: &str, args: &DarwinCleanArgs, needs_sudo: bool) -> Result<u64> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Ok(0);
    }

    let mut total_freed = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            total_freed += safe_remove(&entry_path.to_string_lossy(), args, needs_sudo)?;
        }
    }

    Ok(total_freed)
}

#[cfg(target_os = "macos")]
fn safe_remove(path_str: &str, args: &DarwinCleanArgs, needs_sudo: bool) -> Result<u64> {

    let path = Path::new(path_str);
    if !path.exists() {
        return Ok(0);
    }

    let size = get_size(path)?;
    if size == 0 && path.is_dir() {
        return Ok(0);
    }
    
    let size_human = bytes_to_human(size);

    if args.dry {
        println!("  {} {} ({})", Paint::new(ICON_DRY_RUN).fg(YELLOW).bold(), path_str, size_human);
        return Ok(size); // For potential size calculation
    }

    if args.ask {
        use inquire::Confirm;
        if !Confirm::new(&format!("Remove {} ({})?", path_str, size_human))
            .with_default(false)
            .prompt()? 
        {
            return Ok(0);
        }
    }

    info!("Removing {} ({})", path_str, size_human);
    
    if needs_sudo {
        let result = nh_core::command::Command::new("rm")
           .args(["-rf", path_str])
           .elevate(Some(ElevationStrategy::Auto))
           .message(&format!("Removing protected path {}", path_str))
           .run();
        
        if let Err(e) = result {
            warn!("Failed to remove protected path {}: {}", path_str, e);
            return Ok(0);
        }
    } else {
        if path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(path) {
                // If it fails, try a forced rm -rf as a fallback
                let _ = std::process::Command::new("rm")
                    .args(["-rf", path_str])
                    .status();
                
                if path.exists() {
                    warn!("Failed to remove {}: {}", path_str, e);
                    return Ok(0);
                }
            }
        } else {
            if let Err(e) = std::fs::remove_file(path) {
                warn!("Failed to remove {}: {}", path_str, e);
                return Ok(0);
            }
        }
    }

    println!("  {} {} ({})", Paint::new(ICON_SUCCESS).fg(GREEN).bold(), path_str, size_human);
    Ok(size)
}

#[cfg(target_os = "macos")]
fn get_size(path: &Path) -> Result<u64> {
    let mut total_size = 0;
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    total_size += get_size(&path)?;
                } else if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
    } else if let Ok(metadata) = path.metadata() {
        total_size = metadata.len();
    }
    Ok(total_size)
}

use nh_core::util::bytes_to_human;

#[cfg(test)]
mod tests {
     // use super::*; 
}
