# Build the Rust manager / 编译 Rust 管理器

The manager is a Windows x64 Rust/GPUI application. A normal build does not require Python, CUDA, Vulkan or NGX SDKs, game files, or the historical `development/` tree. It does require the native Windows compiler and SDK used by GPUI and Windows resources.

管理器采用 Rust/GPUI。普通编译不依赖 Python、CUDA、Vulkan/NGX SDK、游戏文件或历史开发目录；需要 Visual Studio C++ 生成工具和 Windows SDK。

## Prerequisites

- Windows 10/11 x64; Visual Studio 2022 Build Tools with **Desktop development with C++**, an x64 MSVC toolset, and Windows SDK.
- Rustup. `rust-toolchain.toml` pins the compiler, target, Clippy and rustfmt; do not replace this with an untested nightly toolchain.
- Use a Developer PowerShell for Visual Studio. Network access is needed for Cargo dependencies and the initial pinned downloader resource. Administrator elevation is not required for normal compilation.

```powershell
git clone https://github.com/pandaligx/RTX-FG-Manager.git
cd RTX-FG-Manager
./tools/prepare-build.ps1
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

Output: `target/x86_64-pc-windows-msvc/release/RTXManager.exe`. The result is **unsigned**. Release signing is separate; never rebuild over a publisher-signed release. Normal debug and release builds embed only the pinned aria2 executable and its configuration, not game DLLs.

`prepare-build.ps1` fetches the unchanged publisher-signed aria2 binary identified by size and SHA-256 in `tools/build-resources.json`. This makes the manager's input reproducible; **it does not claim that this historical aria2 binary can be reproduced from source**. Its original build/source records were not retained; see `THIRD_PARTY_NOTICES.txt`. Bootstrap may use Windows HTTPS download once, because aria2 cannot download itself before it exists. Once prepared, resource downloads use the verified aria2 executable. Existing mismatched files cause a clear error and are not overwritten automatically.

## Optional tests using released, signed binaries

普通 `cargo test` 运行不依赖真实补丁的逻辑和文件系统回归。真实签名 DLL/EXE、旧版升级和专项缓存回归单独启用，不能把默认测试通过描述成这些测试或实体显卡游戏验收通过。

```powershell
./tools/prepare-fixtures.ps1
./tools/prepare-fixtures.ps1 -VerifyOnly
cargo test --locked --features fixture-tests
cargo clippy --locked --all-targets --features fixture-tests -- -D warnings
```

`tests/fixture-manifest.json` pins historical release resources and their exact extracted bytes. Files are stored only in ignored `tests/fixtures/runtime/`. Preparation downloads existing published artifacts, not arbitrary game folders. The Streamline fixture is an optional upstream SDK download of approximately 218 MiB; it retains its upstream license. No test downloads network resources automatically. Missing, changed, or unavailable fixtures fail explicitly. Do not enable `fixture-tests` for a distributable release: it embeds historical DLL fixtures for tests.

## Optional native GPU probes

The UI does not need the C++ synthetic GPU probes. They are a **maintainer-only extension**, separate from the complete public manager build. Their additional native sources are retained in the maintainer's workspace and are not part of this source release; this Rust manager release does not publish the DLL bridge/patch implementation. Enabling the feature in a plain public clone intentionally fails with an explicit explanation. Maintainers who already have the extra `rust/native` sources must also supply SDK headers under the applicable upstream licenses; proprietary NGX headers are not included in this source repository.

```powershell
$env:RTXFG_VULKAN_INCLUDE = 'D:\SDKs\Vulkan-Headers\include'
$env:RTXFG_NGX_INCLUDE = 'D:\SDKs\NGX\include'
# Maintainer workspace only: requires the additional native sources.
cargo build --locked --features native-probes,fixture-tests
```

`RTXFG_NGX_INCLUDE` must contain `nvsdk_ngx_params.h` and its required `nvsdk_ngx_defs.h`; `RTXFG_VULKAN_INCLUDE` must contain `vulkan/vulkan.h`. `native-probes` alone can inspect an explicitly supplied, recognized game deployment; the historical default diagnostic payload requires `fixture-tests`. Builds without `native-probes` return an explicit unsupported-build error for the internal native self-test commands. Native tests need real Windows GPU/driver support; compilation and filesystem tests do not establish hardware compatibility.

## Building on Linux as a Wine prefix manager / Linux 版（Wine 前缀管理器）

The Linux build is not a native port of the Windows API. It manages a Wine
prefix: games live in `<prefix>/drive_c/`, and the manager edits the Windows
registry directly in `<prefix>/system.reg` (HKLM) and `<prefix>/user.reg`
(HKCU). Deploying, replacing and removing files works as it does on Windows, and
registry-backed features edit those `.reg` files. Anything with no honest Wine
equivalent fails with a clear message instead of reporting false success.

Linux 版不是 Windows API 的原生移植，而是 **Wine 前缀管理器**：游戏位于
`<prefix>/drive_c/`，注册表通过直接编辑 `<prefix>/system.reg`（HKLM）与
`<prefix>/user.reg`（HKCU）生效。

Requirements: a Linux x64 host, Rustup with the `x86_64-unknown-linux-gnu`
target, and the following distribution packages (also the CI list):

```bash
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config ca-certificates curl wget file \
  libfreetype-dev libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libxcb-xkb-dev libwayland-dev squashfs-tools desktop-file-utils zsync
```

`clang` is not required: the only `bindgen` in the dependency graph is
macOS-gated. Neither is `libvulkan-dev`, because the renderer dlopens
`libvulkan.so.1` at runtime. A working Vulkan ICD is required at runtime and
comes from the host (`mesa-vulkan-drivers`, or `lavapipe` as a software
fallback). Never bundle a GPU driver or a Vulkan ICD into the AppImage.

`.cargo/config.toml` pins `build.target = "x86_64-pc-windows-msvc"` to keep the
Windows build reproducible. Cargo documents that `--target` overrides it, so a
Linux build **must** pass the target explicitly:

```bash
rustup target add x86_64-unknown-linux-gnu
cargo test  --offline --target x86_64-unknown-linux-gnu
cargo clippy --offline --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
cargo build --release --target x86_64-unknown-linux-gnu
```

Output: `target/x86_64-unknown-linux-gnu/release/RTXManager`. Note the target
subdirectory: it is not `target/release/`.

The downloader differs by platform. The Windows build embeds the verified,
publisher-signed `aria2c.exe` from `tools/build-resources.json`, so it needs no
system tool. The Linux build downloads with the bundled `reqwest` client, so it
needs neither. Fetch the Windows resource with `tools/prepare-build.ps1`; the
Linux build does not require it.

### Wine prefix selection

The manager resolves its prefix in this order: the prefix chosen in the UI, then
`RTXFG_PREFIX`, then `WINEPREFIX`, then `~/.wine`. A non-prefix directory is
rejected with a clear error. Use `--data-dir` to override the settings/library
location, which defaults to `$XDG_DATA_HOME/rtx-fg-manager`
(`~/.local/share/rtx-fg-manager`); extracted runtime files use
`$XDG_CACHE_HOME/rtx-fg-manager/runtime`.

### What the Linux build cannot do

These are reported explicitly, never silently skipped:

- **Authenticode signature trust.** `--signature` and the updater's publisher
  check report `linux-not-verifiable`. The embedded `FileVersion`/`InternalName`
  are still read in pure Rust, and the SHA-256 and size checks are unchanged, so
  update downloads stay integrity-checked. A publisher identity, however, cannot
  be established outside Windows.
- **UAC elevation.** A Wine prefix is user-owned, so `ensure_admin` is a no-op
  and GPU-name changes apply in-process.
- **HAGS / hardware-accelerated GPU scheduling.** A Wine prefix exposes no such
  scheduler; the UI shows "无法确认" and the settings deep-link fails with an
  explanation.
- **Windows Error Reporting event log.** `wevtutil.exe` has no prefix
  equivalent; diagnostic exports record a note and continue.
- **Windows settings URIs.** `ms-settings:` links are rejected with an error.

The GPU display-name feature *is* implemented: a Wine prefix stores the display
adapter database as text (`System\ControlSet00N\Enum\PCI\VEN_10DE&...` with
`DeviceDesc`, plus `Control\Class\{4D36E968-...}` with `DriverDesc`), so the
existing backup/restore record schema is reused against those keys.

### AppImage

```bash
cargo build --locked --release --target x86_64-unknown-linux-gnu
packaging/build-appimage.sh
```

The script assembles the AppDir manually and runs `appimagetool` (override its
location with `APPIMAGETOOL=`). It requires `file`, `mksquashfs` and
`desktop-file-validate` on `PATH`.

`usr/lib` is intentionally **empty**. The release binary's verified `DT_NEEDED`
entries are only `libxkbcommon`, `libxkbcommon-x11`, `libxcb`, `libgcc_s`,
`libm`, `libc` and the loader. `--as-needed` even drops `libfontconfig` and
`libfreetype`, which the GUI stack dlopens rather than links. The AppImage
excludelist forbids redistributing glibc, `libgcc_s`, `libstdc++` and the
graphics-driver libraries, and lists `libxcb`/`libfontconfig` too, so the host
supplies them. **Never bundle a Vulkan ICD or a GPU driver**: `wgpu` dlopens
`libvulkan.so.1` and enumerates `/usr/share/vulkan/icd.d`, so a working host
driver (`mesa-vulkan-drivers`, or `lavapipe` as a software fallback) is required
at runtime.

`.github/workflows/build-linux.yml` builds on **ubuntu-22.04** on purpose: an
AppImage cannot run on a distribution older than the one that built it, so a
newer runner would silently drop support for older distributions. It also sets
`APPIMAGE_EXTRACT_AND_RUN=1`, because GitHub runners do not provide FUSE, and
passes `--locked` rather than `--offline`, since the repository vendors no
registry.



## Source and CI boundaries

Keep `Cargo.lock`, `.cargo/config.toml`, Rust sources/assets, current JSON language resources, and `vendor/gpui_fast_windows` together. GPUI Kit 0.7.1 is pinned to commit `4921e5b69834e45592c9772c57f39a29ada7de2f`; its `gpui-fast` feature selects the Fast backend, resolved to crates.io 0.1.2. Shared GPUI Pre support crates remain pinned to 0.3.8. The Windows vendor patch starts from the published Fast 0.1.2 crate and retains the manager's first-frame, DPI and font compatibility changes. These retained changes do not establish a new Windows 10 or physical multi-monitor test result. The former Zed Windows and sum-tree patches are no longer build inputs or part of this source export.

The GPUI Pre 0.3.8 sum-tree, ztracing, ztracing-macro and zlog packages are Apache-2.0, including their recorded Zed source revision. The former GPL-only tracing-chain workaround is therefore unnecessary. Preserve the actual upstream license files and local change notes.

The 21 offline theme sets in `rust/assets/themes` are copied from the same pinned Kit commit. Their 36 variants plus Kit's two defaults provide 38 color variants. The sidebar uses Kit's `Command` component for one searchable list and ephemeral hover/keyboard previews. Only confirmation persists the selected name and mode; cancellation restores saved preferences, and a generation token rejects stale deferred callbacks. Suspend system-theme polling during previews. Keep independent light/dark selections through `Theme::update`; unavailable selections fall back to the matching mode's default. Preserve theme source attribution, license and embedded JSON files with the Rust source.

The GitHub Windows workflow builds an **unsigned test artifact**, never signs, publishes, or replaces the release update manifest. Signed EXE and game DLL releases remain separate from source commits.

Secrets, certificates/private keys, local logs, game binaries, historical development outputs and runtime caches must not be committed. The published source export uses an explicit file allowlist in addition to `.gitignore`. DLL ownership records used for safe cleanup are required application data and are not disposable build caches.
