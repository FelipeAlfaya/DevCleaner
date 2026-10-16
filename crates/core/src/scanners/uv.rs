use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{Category, Risk};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{cache_dir, home_dir, local_data_dir};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let cache = resolve_cache();
    let mut nodes = Vec::new();
    if cache.exists() {
        let jail = cache.clone();
        if let Ok(entries) = std::fs::read_dir(&cache) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Uv,
                    None,
                    entry.file_name().to_string_lossy().into(),
                    path,
                    jail.clone(),
                    "Entrada de cache uv",
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
        Category::Uv,
        available,
        if available {
            None
        } else {
            Some("Cache uv não encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_cache() -> PathBuf {
    if let Some(local) = local_data_dir() {
        let p = local.join("uv");
        if p.exists() {
            return p;
        }
    }
    cache_dir()
        .map(|c| c.join("uv"))
        .or_else(|| home_dir().map(|h| h.join(".cache").join("uv")))
        .unwrap_or_else(|| PathBuf::from(".cache/uv"))
}
