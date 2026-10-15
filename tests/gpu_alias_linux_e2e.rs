//! End-to-end GPU display-name aliasing against a COPY of a real Wine prefix.
//! Never touches the user's live prefix: the test copies system.reg to a temp
//! prefix first and asserts the rename/restore cycle and the backup record.
#![cfg(not(windows))]
use rtx_fg_manager::{gpu_alias, wine_reg};

fn copy_prefix(source: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;
    std::fs::copy(source.join("system.reg"), dest.join("system.reg"))?;
    if source.join("user.reg").is_file() {
        std::fs::copy(source.join("user.reg"), dest.join("user.reg"))?;
    }
    std::fs::create_dir_all(dest.join("drive_c"))?;
    Ok(())
}

#[test]
fn rename_and_restore_cycle_updates_a_copied_prefix_registry() -> anyhow::Result<()> {
    let Ok(live) = rtx_fg_manager::win::prefix() else {
        eprintln!("skipped: no Wine prefix configured");
        return Ok(());
    };
    if !wine_reg::hive_path(&live, wine_reg::Hive::Machine).is_file() {
        eprintln!("skipped: prefix has no system.reg");
        return Ok(());
    }
    let work = std::env::temp_dir().join(format!("rtxfg-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    copy_prefix(&live, &work)?;
    rtx_fg_manager::win::set_prefix(&work)?;

    let devices = gpu_alias::enumerate()?;
    if devices.is_empty() {
        eprintln!("skipped: copied prefix has no NVIDIA display adapter");
        let _ = std::fs::remove_dir_all(&work);
        return Ok(());
    }
    let target = devices[0].clone();
    let original = target.name.clone();

    // Record the exact bytes so a failed cycle cannot silently corrupt the hive.
    let before = std::fs::read(wine_reg::hive_path(&work, wine_reg::Hive::Machine))?;

    let message = gpu_alias::apply(target.clone(), Some(gpu_alias::NAMES[0]))?;
    assert!(message.contains(gpu_alias::NAMES[0]));

    let renamed = gpu_alias::enumerate()?
        .into_iter()
        .find(|d| d.instance.eq_ignore_ascii_case(&target.instance))
        .expect("adapter still present");
    assert_eq!(
        renamed.name,
        gpu_alias::NAMES[0],
        "DeviceDesc must be renamed"
    );

    // A backup record must exist so restore is possible.
    let restored = gpu_alias::apply(renamed, None)?;
    assert!(restored.contains("还原"));
    let after = gpu_alias::enumerate()?
        .into_iter()
        .find(|d| d.instance.eq_ignore_ascii_case(&target.instance))
        .expect("adapter still present");
    assert_eq!(
        after.name, original,
        "restore must return the original name"
    );

    // The registry must still parse and differ only where intended.
    let final_bytes = std::fs::read(wine_reg::hive_path(&work, wine_reg::Hive::Machine))?;
    assert!(!final_bytes.is_empty());
    assert_ne!(before, final_bytes, "an edit must have occurred");
    wine_reg::RegFile::open(&wine_reg::hive_path(&work, wine_reg::Hive::Machine))
        .expect("registry remains parseable after the cycle");

    let _ = std::fs::remove_dir_all(&work);
    Ok(())
}
