use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{Category, DeleteStrategy, Risk};
use crate::scanners::common::{action_node, emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{dir_size, home_dir};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let mut nodes = Vec::new();

    if let Some(build_cache) = resolve_build_cache() {
        if build_cache.exists() {
            let size = dir_size(&build_cache);
            nodes.push(action_node(
                Category::Go,
                "Cache de build Go".into(),
                build_cache.display().to_string(),
                size,
                "Limpeza via go clean -cache",
                DeleteStrategy::GoCleanCache,
                true,
            ));
        }
    }

    if let Some(mod_cache) = resolve_mod_cache() {
        if mod_cache.exists() {
            if let Ok(entries) = std::fs::read_dir(&mod_cache) {
                let jail = mod_cache.clone();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_symlink() {
                        continue;
                    }
                    let is_dir = path.is_dir();
                    nodes.push(path_node(
                        Category::Go,
                        None,
                        entry.file_name().to_string_lossy().into(),
                        path,
                        jail.clone(),
                        "Módulo em cache (pkg/mod)",
                        Risk::Safe,
                        true,
                        is_dir,
                    ));
                }
            }
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Go,
        available,
        if available {
            None
        } else {
            Some("Cache Go não encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_build_cache() -> Option<PathBuf> {
    std::env::var("GOCACHE")
        .map(PathBuf::from)
        .ok()
        .or_else(|| home_dir().map(|h| h.join(".cache").join("go-build")))
}

fn resolve_mod_cache() -> Option<PathBuf> {
    std::env::var("GOPATH")
        .map(|g| PathBuf::from(g).join("pkg").join("mod"))
        .ok()
        .or_else(|| home_dir().map(|h| h.join("go").join("pkg").join("mod")))
}
