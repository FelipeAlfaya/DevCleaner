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

    let repo = resolve_repo();
    let mut nodes = Vec::new();
    if repo.exists() {
        if let Ok(entries) = std::fs::read_dir(&repo) {
            let jail = repo.clone();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Maven,
                    None,
                    entry.file_name().to_string_lossy().into(),
                    path,
                    jail.clone(),
                    "Grupo Maven (~/.m2/repository) — pode incluir artefatos instalados localmente",
                    Risk::Review,
                    false,
                    is_dir,
                ));
            }
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Maven,
        available,
        if available {
            None
        } else {
            Some("Repositório Maven local não encontrado".into())
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
        Category::Maven,
        node.id,
        path,
        jail_root.clone(),
        "Coordenada Maven",
        Risk::Review,
        false,
        3,
    );
    send_nodes(tx, children);
}

fn resolve_repo() -> PathBuf {
    home_dir()
        .map(|h| h.join(".m2").join("repository"))
        .unwrap_or_else(|| PathBuf::from(".m2/repository"))
}
