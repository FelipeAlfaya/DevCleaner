use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::model::{Category, NodeTarget, Risk, ScanNode};
use crate::scanners::common::{emit_category_summary, list_children, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{age_days, temp_dirs};

pub fn scan(settings: &Settings, tx: &Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let mut nodes = Vec::new();
    for root in temp_dirs() {
        if !root.is_dir() {
            continue;
        }
        let jail = root.clone();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                if let Some(age) = age_days(&path) {
                    if age < settings.temp_min_age_days {
                        continue;
                    }
                }
                let is_dir = path.is_dir();
                nodes.push(path_node(
                    Category::Temp,
                    None,
                    format!("{} · {}", entry.file_name().to_string_lossy(), root.display()),
                    path,
                    jail.clone(),
                    &format!(
                        "Temporário com idade mínima de {} dias",
                        settings.temp_min_age_days
                    ),
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
        Category::Temp,
        available,
        if available {
            None
        } else {
            Some("Nenhum temporário elegível encontrado".into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

pub fn expand(node: &ScanNode, settings: &Settings, tx: &Sender<crate::model::ScanEvent>) {
    let _ = settings;
    let NodeTarget::Path { path, jail_root } = &node.target else {
        return;
    };
    let children = list_children(
        Category::Temp,
        node.id,
        path,
        jail_root.clone(),
        "Conteúdo temporário",
        Risk::Safe,
        true,
        1,
    );
    send_nodes(tx, children);
}
