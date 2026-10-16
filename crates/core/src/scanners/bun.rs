use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{Category, Risk};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::home_dir;

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
                    Category::Bun,
                    None,
                    entry.file_name().to_string_lossy().into(),
                    path,
                    jail.clone(),
                    "Entrada de cache Bun",
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
        Category::Bun,
        available,
        if available {
            None
        } else {
            Some("Cache Bun não encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_cache() -> PathBuf {
    home_dir()
        .map(|h| h.join(".bun").join("install").join("cache"))
        .unwrap_or_else(|| PathBuf::from(".bun/install/cache"))
}
