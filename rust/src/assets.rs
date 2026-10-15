use crate::core;
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub struct Embedded {
    pub name: &'static str,
    pub sha256: &'static str,
    pub size: usize,
    pub compressed: &'static [u8],
}
include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
pub fn bytes(name: &str) -> Result<Vec<u8>> {
    let r = EMBEDDED
        .iter()
        .find(|r| r.name == name)
        .context("安装包文件缺失")?;
    let mut bytes = Vec::with_capacity(r.size);
    flate2::read::ZlibDecoder::new(r.compressed)
        .take(r.size as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() == r.size && core::hash(&bytes) == r.sha256,
        "内置资源损坏"
    );
    Ok(bytes)
}
/// Per-user cache directory for extracted runtime files.
///
/// Windows uses `%LOCALAPPDATA%\RTXFGManager\runtime-rust`. Linux follows the
/// XDG base-directory spec (`$XDG_CACHE_HOME/rtx-fg-manager/runtime`, falling
/// back to `~/.cache/...`), which is the conventional, user-writable location.
pub fn cache_root() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        Ok(
            PathBuf::from(std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA unavailable")?)
                .join("RTXFGManager")
                .join("runtime-rust"),
        )
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".cache"))
            })
            .context("无法确定缓存目录：请设置 XDG_CACHE_HOME 或 HOME")?;
        Ok(base.join("rtx-fg-manager").join("runtime"))
    }
}
#[cfg_attr(not(windows), allow(dead_code))]
fn write_resource(path: &Path, bytes: &[u8]) -> Result<()> {
    let path = core::no_links(path)?;
    let parent = path.parent().context("无效资源路径")?;
    // Closing during a startup download may terminate this worker. Only publish
    // the verified resource after its complete contents have reached the file;
    // a stopped extraction must never leave a partial executable at its name.
    let mut stage = tempfile::NamedTempFile::new_in(parent)?;
    stage.write_all(bytes)?;
    stage.as_file().sync_all()?;
    core::no_links(&path)?;
    stage.persist_noclobber(&path)?;
    Ok(())
}
/// Locates the `aria2.conf` that belongs to the executable returned by
/// [`tool`].
///
/// On Windows the conf sits next to `aria2c.exe`. On Linux the executable is
/// the system's `aria2c` (often `/usr/bin/aria2c`), so its sibling would be
/// `/usr/bin/aria2.conf`, which is not where the bundled config lives. This
/// indirection keeps `transfer.rs` platform-neutral.
#[cfg(windows)]
pub fn tool_conf(executable: &Path) -> PathBuf {
    executable.with_file_name("aria2.conf")
}

/// Resolves the external downloader used by `transfer.rs`, Windows only.
///
/// The Windows build extracts the verified, embedded `aria2c.exe` and
/// `aria2.conf` into the cache, so the shipped bytes are reproducible and no
/// system tool is needed. Linux does not call this at all: `transfer.rs`
/// downloads with the bundled `reqwest` client there, because requiring a system
/// `aria2c` would make downloads impossible on machines that lack it.
#[cfg(windows)]
pub fn tool() -> Result<PathBuf> {
    let root = cache_root()?.join(
        "aria2-".to_owned()
            + &EMBEDDED
                .iter()
                .find(|r| r.name == "app/tools/aria2c.exe")
                .unwrap()
                .sha256[..16],
    );
    core::no_links(&root)?;
    std::fs::create_dir_all(&root)?;
    let _lock = crate::win::resource_lock(&root)?;
    for name in ["aria2c.exe", "aria2.conf"] {
        let path = core::no_links(&root.join(name))?;
        let resource = format!("app/tools/{name}");
        let item = EMBEDDED.iter().find(|r| r.name == resource).unwrap();
        if path.exists() {
            ensure!(
                core::digest(&path)? == item.sha256,
                "下载工具已被修改，未执行"
            );
        } else {
            write_resource(&path, &bytes(&resource)?)?;
        }
    }
    Ok(root.join("aria2c.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_commit_never_overwrites_existing_files_or_leaves_failed_stage() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("aria2c.exe");
        std::fs::write(&path, b"unknown existing file")?;
        assert!(write_resource(&path, b"replacement is forbidden").is_err());
        assert_eq!(std::fs::read(&path)?, b"unknown existing file");
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
        let new_path = directory.path().join("aria2.conf");
        write_resource(&new_path, b"complete verified resource")?;
        assert_eq!(std::fs::read(&new_path)?, b"complete verified resource");
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 2);
        Ok(())
    }
}
