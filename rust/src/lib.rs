//! Cross-platform Rust manager core.
//!
//! The Windows build talks to the live Win32 API. The Linux build never runs as
//! a Windows process: it manages a Wine prefix on disk, so `drive_c` is an
//! ordinary directory tree and the registry is text (`system.reg` / `user.reg`).
//!
//! To avoid threading `#[cfg]` through ~22k lines, the platform-specific
//! modules are selected by `#[path]` while keeping their PUBLIC NAMES identical.
//! Every existing call site (`crate::win::game_lock`, `crate::gpu_alias::enumerate`,
//! ...) therefore compiles unchanged on both targets, and the Windows files are
//! still compiled verbatim on Windows.

pub mod assets;
pub mod cache;
pub mod cleanup;
pub mod cloud;
pub mod core;
pub mod delta;
pub mod deployment;
pub mod diagnostics;
pub mod encore;
pub mod encore_upgrade;
pub mod i18n;
pub mod jsonc;
pub mod pe_version;
pub mod platform;
pub mod preferences;
pub mod presets;
pub mod release_notes;
pub mod rtxmfg;
pub mod rtxmfg_upgrade;
pub mod scanner;
pub mod transfer;
pub mod transfusion;
pub mod updater;
pub mod wine_reg;

// Platform-selected modules. The Windows sources are untouched; the `_linux`
// siblings expose the same public API so callers stay identical.
#[cfg(windows)]
#[path = "win.rs"]
pub mod win;
#[cfg(not(windows))]
#[path = "win_linux.rs"]
pub mod win;

#[cfg(windows)]
#[path = "elevation.rs"]
pub mod elevation;
#[cfg(not(windows))]
#[path = "elevation_linux.rs"]
pub mod elevation;

#[cfg(windows)]
#[path = "game_icons.rs"]
pub mod game_icons;
#[cfg(not(windows))]
#[path = "game_icons_linux.rs"]
pub mod game_icons;

#[cfg(windows)]
#[path = "gpu_alias.rs"]
pub mod gpu_alias;
#[cfg(not(windows))]
#[path = "gpu_alias_linux.rs"]
pub mod gpu_alias;

#[cfg(windows)]
#[path = "hags.rs"]
pub mod hags;
#[cfg(not(windows))]
#[path = "hags_linux.rs"]
pub mod hags;

#[cfg(windows)]
#[path = "selftest.rs"]
pub mod selftest;
#[cfg(not(windows))]
#[path = "selftest_linux.rs"]
pub mod selftest;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
