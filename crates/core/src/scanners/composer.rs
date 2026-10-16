use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::i18n::Msg;
use crate::model::{Category, Risk};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{cache_dir, home_dir, run_command};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let cache = resolve_cache();
    let mut nodes = Vec::new();
    if cache.exists() {
        if let Ok(entries) = std::fs::read_dir(&cache) {
            let jail = cache.clone();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Composer,
                    None,
                    entry.file_name().to_string_lossy().to_string(),
                    path,
                    jail.clone(),
                    Msg::ComposerEntry,
                    Risk::Safe,
                    true,
                    is_dir,
                ));
            }
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Composer,
        available,
        if available {
            None
        } else {
            Some(Msg::ComposerNone.into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_cache() -> PathBuf {
    if let Some(out) = run_command(
        "composer",
        &["config", "cache-dir"],
        std::time::Duration::from_secs(5),
    ) {
        let p = PathBuf::from(out.trim());
        if p.exists() {
            return p;
        }
    }
    cache_dir()
        .map(|c| c.join("composer"))
        .or_else(|| home_dir().map(|h| h.join(".cache").join("composer")))
        .unwrap_or_else(|| PathBuf::from(".cache/composer"))
}
