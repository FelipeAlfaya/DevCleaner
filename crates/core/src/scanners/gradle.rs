use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::model::{Category, NodeTarget, Risk, ScanNode};
use crate::scanners::common::{emit_category_summary, list_children, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::home_dir;

pub fn scan(settings: &Settings, tx: &Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let caches = resolve_caches();
    let mut nodes = Vec::new();
    if caches.exists() {
        if let Ok(entries) = std::fs::read_dir(&caches) {
            let jail = resolve_gradle_home();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Gradle,
                    None,
                    entry.file_name().to_string_lossy().into(),
                    path,
                    jail.clone(),
                    "Grupo de cache Gradle",
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
        Category::Gradle,
        available,
        if available {
            None
        } else {
            Some("Cache Gradle não encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

pub fn expand(node: &ScanNode, _settings: &Settings, tx: &Sender<crate::model::ScanEvent>) {
    let NodeTarget::Path { path, jail_root } = &node.target else {
        return;
    };
    let children = list_children(
        Category::Gradle,
        node.id,
        path,
        jail_root.clone(),
        "Artefato Gradle",
        Risk::Safe,
        true,
        2,
    );
    send_nodes(tx, children);
}

fn resolve_gradle_home() -> PathBuf {
    std::env::var("GRADLE_USER_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".gradle")))
        .unwrap_or_else(|| PathBuf::from(".gradle"))
}

fn resolve_caches() -> PathBuf {
    resolve_gradle_home().join("caches")
}
