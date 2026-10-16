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

    let mut nodes = Vec::new();
    for root in pip_roots() {
        if !root.exists() {
            continue;
        }
        let jail = root.clone();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Pip,
                    None,
                    format!("{} ({})", entry.file_name().to_string_lossy(), root.display()),
                    path,
                    jail.clone(),
                    "Wheel/sdist em cache pip",
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
        Category::Pip,
        available,
        if available {
            None
        } else {
            Some("Cache pip não encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn pip_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(c) = cache_dir() {
        roots.push(c.join("pip"));
    }
    if let Some(local) = local_data_dir() {
        roots.push(local.join("pip").join("Cache"));
    }
    if let Some(h) = home_dir() {
        roots.push(h.join(".cache").join("pip"));
    }
    roots.sort();
    roots.dedup();
    roots
}
