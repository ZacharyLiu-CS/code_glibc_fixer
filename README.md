# code_glibc_fixer

A Rust CLI tool that fixes VSCode Server glibc version issues on systems with old glibc (e.g. tlinux2/glibc-2.17).

## Background

Since early 2024, VSCode upgraded its Electron framework to v27, raising the minimum required glibc version to 2.28. On tlinux2 environments (glibc-2.17), this causes the VSCode Server to refuse to start.

This tool automates the fix described in [the iwiki doc](https://iwiki.woa.com/p/4018161950):

1. Downloads `glibc-2.38` and `libstdc++-12.3.1` RPMs from Tencent mirrors and extracts them to `/root/usrlibs`
2. Uses `patchelf` to set a custom interpreter and rpath on the VSCode Server `node` binary
3. Patches `check-requirements.sh` to bypass the glibc version check

## Usage

### Step 1: Prepare (download libraries + install tools)

```bash
./code_glibc_fixer prepare
```

This will:
- Install `patchelf` via `yum` if not present
- Download glibc and libstdc++ RPMs from `mirrors.tencent.com`
- Extract them to `/root/usrlibs`

Only needs to be run once.

### Step 2: Fix existing VSCode Server installations

```bash
# Fix all entries under ~/.vscode-server/bin
./code_glibc_fixer fix

# Fix a specific directory
./code_glibc_fixer fix ~/.vscode-server/bin/<hash>
```

### Step 3: Watch mode (auto-fix new installations)

```bash
# Watch ~/.vscode-server/bin for new entries and fix them automatically
./code_glibc_fixer fix --watch ~/.vscode-server/bin
```

When in watch mode:
- All existing entries under the directory are fixed first
- New directories created by VSCode Server are detected via inotify
- The tool waits for the `node` binary to appear (up to 120s), then applies the fix automatically

## Building

```bash
cargo build --release
# Binary at: target/release/code_glibc_fixer
```

## What it does

### `prepare`
- Installs `patchelf`, `rpm2cpio`, `cpio`, `wget` if missing
- Downloads RPMs from `https://mirrors.tencent.com/tlinux/4/BaseOS/x86_64/os/Packages/`
- Extracts them to `/root/usrlibs`

### `fix`
For each vscode-server `<hash>` directory under the target path:
1. **Patches the `node` binary** using patchelf:
   ```
   patchelf --set-interpreter /root/usrlibs/lib64/ld-linux-x86-64.so.2 \
            --set-rpath /root/usrlibs/usr/lib64:/root/usrlibs/lib64 \
            ~/.vscode-server/bin/<hash>/node
   ```
2. **Patches `check-requirements.sh`** by injecting override lines:
   ```sh
   found_required_glibc=1
   found_required_glibcxx=1
   ```

### `fix --watch <dir>`
- Runs the fix on all existing entries first
- Then monitors `<dir>` using Linux inotify for `IN_CREATE` and `IN_MOVED_TO` events
- Automatically applies the fix when a new directory is detected
