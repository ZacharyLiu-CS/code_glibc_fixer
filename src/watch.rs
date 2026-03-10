use anyhow::{Context, Result};
use inotify::{Inotify, WatchMask, EventMask};
use std::path::Path;

use crate::fix;

/// Watch `bin_dir` for new subdirectory creation events.
/// When a new directory is created, trigger the fix flow on it.
pub fn run(bin_dir: &Path) -> Result<()> {
    println!("[watch] Watching {} for new vscode-server entries...", bin_dir.display());

    let mut inotify = Inotify::init()
        .context("Failed to initialize inotify. Are you on Linux?")?;

    // Watch for:
    //   IN_CREATE     - file/dir created
    //   IN_MOVED_TO   - directory moved into watched dir (e.g. rename-based extraction)
    inotify
        .watches()
        .add(bin_dir, WatchMask::CREATE | WatchMask::MOVED_TO)
        .with_context(|| format!("Failed to add inotify watch on {}", bin_dir.display()))?;

    println!("[watch] Watching for new directories. Press Ctrl+C to stop.");

    let mut buffer = [0u8; 4096];
    loop {
        let events = inotify
            .read_events_blocking(&mut buffer)
            .context("Failed to read inotify events")?;

        for event in events {
            // We only care about directories
            if !event.mask.contains(EventMask::ISDIR) {
                continue;
            }

            let name = match event.name {
                Some(n) => n.to_string_lossy().into_owned(),
                None => continue,
            };

            let entry_dir = fix::entry_path(bin_dir, &name);
            println!("[watch] New vscode-server directory detected: {}", entry_dir.display());

            // The directory was just created; vscode may still be writing files into it.
            // Wait a moment to let the extraction finish.
            wait_for_node_binary(&entry_dir);

            println!("[watch] Triggering fix for: {}", entry_dir.display());
            if let Err(e) = fix::fix_entry(&entry_dir) {
                eprintln!("[watch] Error fixing {}: {:#}", entry_dir.display(), e);
            }
        }
    }
}

/// Poll until the `node` binary appears inside the entry directory, or timeout.
/// vscode-server extracts files after creating the directory, so we need to wait.
fn wait_for_node_binary(entry_dir: &Path) {
    let node_bin = entry_dir.join("node");
    let timeout = std::time::Duration::from_secs(120);
    let poll_interval = std::time::Duration::from_millis(500);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if node_bin.exists() {
            // Give it another second to finish writing
            std::thread::sleep(std::time::Duration::from_secs(2));
            return;
        }
        std::thread::sleep(poll_interval);
    }

    println!(
        "[watch] Warning: node binary not found at {} after {}s, proceeding anyway.",
        node_bin.display(),
        timeout.as_secs()
    );
}
