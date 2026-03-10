use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;
use std::fs;

/// Base directory for downloaded/extracted libs
pub const USRLIBS_DIR: &str = "/root/usrlibs";

const GLIBC_RPM_URL: &str =
    "https://mirrors.tencent.com/tlinux/4/BaseOS/x86_64/os/Packages/glibc-2.38-36.tl4.x86_64.rpm";
const LIBSTDCXX_RPM_URL: &str =
    "https://mirrors.tencent.com/tlinux/4/BaseOS/x86_64/os/Packages/libstdc++-12.3.1.4-2.tl4.x86_64.rpm";

pub fn run() -> Result<()> {
    println!("[prepare] Starting preparation...");

    // 1. Install tools
    install_tools()?;

    // 2. Download + extract RPMs
    download_and_extract_rpms()?;

    println!("[prepare] Done. Libraries are ready at {}", USRLIBS_DIR);
    println!("[prepare] You can now run: ./code_glibc_fixer fix");
    Ok(())
}

fn install_tools() -> Result<()> {
    // Check if patchelf is already available
    if is_available("patchelf") {
        println!("[prepare] patchelf already installed, skipping.");
    } else {
        println!("[prepare] Installing patchelf via yum...");
        run_cmd("sudo", &["yum", "install", "-y", "patchelf"])
            .context("Failed to install patchelf. Try: sudo yum install patchelf")?;
        println!("[prepare] patchelf installed.");
    }

    // rpm2cpio and cpio are usually pre-installed; check anyway
    for tool in &["rpm2cpio", "cpio", "wget"] {
        if is_available(tool) {
            println!("[prepare] {} already available.", tool);
        } else {
            println!("[prepare] {} not found, trying to install...", tool);
            let pkg = if *tool == "rpm2cpio" { "rpm" } else { *tool };
            run_cmd("sudo", &["yum", "install", "-y", pkg])
                .with_context(|| format!("Failed to install {}", tool))?;
        }
    }

    Ok(())
}

fn download_and_extract_rpms() -> Result<()> {
    let usrlibs = Path::new(USRLIBS_DIR);
    fs::create_dir_all(usrlibs).context("Failed to create /root/usrlibs")?;

    let glibc_rpm = usrlibs.join("glibc-2.38-36.tl4.x86_64.rpm");
    let libstdcxx_rpm = usrlibs.join("libstdc++-12.3.1.4-2.tl4.x86_64.rpm");

    // Download glibc RPM
    if !glibc_rpm.exists() {
        println!("[prepare] Downloading glibc RPM...");
        run_cmd("wget", &["-O", glibc_rpm.to_str().unwrap(), GLIBC_RPM_URL])
            .context("Failed to download glibc RPM")?;
    } else {
        println!("[prepare] glibc RPM already downloaded, skipping.");
    }

    // Download libstdc++ RPM
    if !libstdcxx_rpm.exists() {
        println!("[prepare] Downloading libstdc++ RPM...");
        run_cmd("wget", &["-O", libstdcxx_rpm.to_str().unwrap(), LIBSTDCXX_RPM_URL])
            .context("Failed to download libstdc++ RPM")?;
    } else {
        println!("[prepare] libstdc++ RPM already downloaded, skipping.");
    }

    // Extract glibc RPM
    let glibc_marker = usrlibs.join(".glibc_extracted");
    if !glibc_marker.exists() {
        println!("[prepare] Extracting glibc RPM...");
        extract_rpm(glibc_rpm.to_str().unwrap(), USRLIBS_DIR)
            .context("Failed to extract glibc RPM")?;
        fs::write(&glibc_marker, "")?;
    } else {
        println!("[prepare] glibc RPM already extracted, skipping.");
    }

    // Extract libstdc++ RPM
    let libstdcxx_marker = usrlibs.join(".libstdcxx_extracted");
    if !libstdcxx_marker.exists() {
        println!("[prepare] Extracting libstdc++ RPM...");
        extract_rpm(libstdcxx_rpm.to_str().unwrap(), USRLIBS_DIR)
            .context("Failed to extract libstdc++ RPM")?;
        fs::write(&libstdcxx_marker, "")?;
    } else {
        println!("[prepare] libstdc++ RPM already extracted, skipping.");
    }

    // Verify key files exist
    let ld_so = usrlibs.join("lib64/ld-linux-x86-64.so.2");
    if !ld_so.exists() {
        anyhow::bail!(
            "Expected {} to exist after extraction, but it's missing. \
             Check that the RPMs were downloaded and extracted correctly.",
            ld_so.display()
        );
    }

    println!("[prepare] Library extraction complete.");
    Ok(())
}

fn extract_rpm(rpm_path: &str, dest_dir: &str) -> Result<()> {
    // rpm2cpio <rpm> | cpio -idmv
    // We run this via shell since piping requires it
    let cmd = format!(
        "cd {} && rpm2cpio {} | cpio -idm",
        dest_dir, rpm_path
    );
    let status = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
        .context("Failed to run rpm2cpio | cpio")?;
    if !status.success() {
        anyhow::bail!("rpm2cpio | cpio failed for {}", rpm_path);
    }
    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run {} {:?}", program, args))?;
    if !status.success() {
        anyhow::bail!("{} {:?} exited with non-zero status", program, args);
    }
    Ok(())
}

fn is_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
