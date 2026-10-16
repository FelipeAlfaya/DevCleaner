use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::i18n::Msg;
use crate::model::{Category, Risk};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::home_dir;

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let packages = resolve_packages();
    let mut nodes = Vec::new();
    if packages.exists() {
        if let Ok(entries) = std::fs::read_dir(&packages) {
            let jail = packages.clone();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                if path.is_dir() {
                    if let Ok(versions) = std::fs::read_dir(&path) {
                        for ver in versions.flatten() {
                            let vpath = ver.path();
                            nodes.push(path_node(
                                Category::NuGet,
                                None,
                                format!(
                                    "{}@{}",
                                    entry.file_name().to_string_lossy(),
                                    ver.file_name().to_string_lossy()
                                ),
                                vpath,
                                jail.clone(),
                                Msg::NugetPackage,
                                Risk::Safe,
                                true,
                                false,
                            ));
                        }
                    }
                }
            }
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::NuGet,
        available,
        if available {
            None
        } else {
            Some(Msg::NugetNone.into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_packages() -> PathBuf {
    home_dir()
        .map(|h| h.join(".nuget").join("packages"))
        .unwrap_or_else(|| PathBuf::from(".nuget/packages"))
}
