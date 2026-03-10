use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

use crate::prepare::USRLIBS_DIR;

/// Fix all vscode-server hash directories under `bin_dir` (e.g. ~/.vscode-server/bin)
pub fn fix_all_in_dir(bin_dir: &Path) -> Result<()> {
    if !bin_dir.exists() {
        println!("[fix] Directory {} does not exist yet, nothing to fix.", bin_dir.display());
        return Ok(());
    }

    let entries = fs::read_dir(bin_dir)
        .with_context(|| format!("Cannot read directory: {}", bin_dir.display()))?;

    let mut fixed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            println!("[fix] Found vscode-server entry: {}", path.display());
            match fix_entry(&path) {
                Ok(_) => fixed += 1,
                Err(e) => eprintln!("[fix] Error fixing {}: {:#}", path.display(), e),
            }
        }
    }

    if fixed == 0 {
        println!("[fix] No vscode-server entries found under {}", bin_dir.display());
    } else {
        println!("[fix] Fixed {} vscode-server entries.", fixed);
    }

    Ok(())
}

/// Fix a single vscode-server hash directory (e.g. ~/.vscode-server/bin/<hash>)
pub fn fix_entry(entry_dir: &Path) -> Result<()> {
    println!("[fix] Fixing entry: {}", entry_dir.display());

    // Step 1: patchelf the node binary
    let node_bin = entry_dir.join("node");
    if node_bin.exists() {
        patch_node_binary(&node_bin).with_context(|| {
            format!("Failed to patchelf node binary at {}", node_bin.display())
        })?;
    } else {
        println!(
            "[fix] WARNING: node binary not found at {}, skipping patchelf.",
            node_bin.display()
        );
    }

    // Step 2: Patch check-requirements.sh
    let check_script = entry_dir.join("bin").join("helpers").join("check-requirements.sh");
    if check_script.exists() {
        patch_check_requirements(&check_script).with_context(|| {
            format!(
                "Failed to patch check-requirements.sh at {}",
                check_script.display()
            )
        })?;
    } else {
        println!(
            "[fix] WARNING: check-requirements.sh not found at {}, skipping.",
            check_script.display()
        );
    }

    println!("[fix] Done fixing: {}", entry_dir.display());
    Ok(())
}

fn patch_node_binary(node_bin: &Path) -> Result<()> {
    let usrlibs = Path::new(USRLIBS_DIR);
    let ld_so = usrlibs.join("lib64/ld-linux-x86-64.so.2");
    let rpath = format!(
        "{}:{}",
        usrlibs.join("usr/lib64").display(),
        usrlibs.join("lib64").display()
    );

    // Check if already patched
    if is_already_patched(node_bin)? {
        println!("[fix] node binary already patched, skipping patchelf.");
        return Ok(());
    }

    println!("[fix] Running patchelf on {}...", node_bin.display());

    let status = Command::new("patchelf")
        .arg("--set-interpreter")
        .arg(ld_so.to_str().unwrap())
        .arg("--set-rpath")
        .arg(&rpath)
        .arg(node_bin)
        .status()
        .context("Failed to run patchelf. Is it installed? Run: ./code_glibc_fixer prepare")?;

    if !status.success() {
        anyhow::bail!("patchelf exited with non-zero status on {}", node_bin.display());
    }

    println!("[fix] patchelf applied successfully to {}", node_bin.display());
    Ok(())
}

/// Check if the node binary has already been patched by looking at its rpath/interpreter
fn is_already_patched(node_bin: &Path) -> Result<bool> {
    let output = Command::new("patchelf")
        .arg("--print-interpreter")
        .arg(node_bin)
        .output();

    match output {
        Ok(out) => {
            let interpreter = String::from_utf8_lossy(&out.stdout);
            Ok(interpreter.contains(USRLIBS_DIR))
        }
        Err(_) => Ok(false), // patchelf not available or error - proceed with patching
    }
}

fn patch_check_requirements(script_path: &Path) -> Result<()> {
    let content = fs::read_to_string(script_path)
        .with_context(|| format!("Cannot read {}", script_path.display()))?;

    let already_patched = content.contains("# patched by code_glibc_fixer");
    if already_patched {
        println!("[fix] check-requirements.sh already patched, skipping.");
        return Ok(());
    }

    // Replace the logic that sets found_required_glibc and found_required_glibcxx
    // Strategy: inject override lines right after the shebang / at the start of the script
    // that force found_required_glibc=1 and found_required_glibcxx=1
    let patched = inject_glibc_override(&content);

    fs::write(script_path, &patched)
        .with_context(|| format!("Cannot write patched {}", script_path.display()))?;

    println!("[fix] Patched check-requirements.sh at {}", script_path.display());
    Ok(())
}

/// Inject override variables right after the shebang line so glibc checks always pass.
fn inject_glibc_override(content: &str) -> String {
    let inject = concat!(
        "# patched by code_glibc_fixer - force glibc/glibcxx checks to pass\n",
        "found_required_glibc=1\n",
        "found_required_glibcxx=1\n",
    );

    // Find shebang line
    if let Some(newline_pos) = content.find('\n') {
        let shebang = &content[..=newline_pos];
        let rest = &content[newline_pos + 1..];
        format!("{}{}{}", shebang, inject, rest)
    } else {
        // No newline found, just prepend
        format!("{}\n{}", inject, content)
    }
}

/// Build the path to a vscode-server entry given the bin dir and a directory name (hash)
pub fn entry_path(bin_dir: &Path, name: &str) -> PathBuf {
    bin_dir.join(name)
}
