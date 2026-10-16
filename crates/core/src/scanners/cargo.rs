use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::i18n::Msg;
use crate::model::{Category, NodeTarget, Risk, ScanNode};
use crate::scanners::common::{emit_category_summary, list_children, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::home_dir;

pub fn scan(settings: &Settings, tx: &Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let cargo_home = resolve_cargo_home();
    let mut nodes = Vec::new();
    for sub in ["registry/cache", "registry/src", "git/checkouts", "git/db"] {
        let path = cargo_home.join(sub);
        if path.exists() {
            nodes.push(path_node(
                Category::Cargo,
                None,
                sub.replace('/', " / "),
                path.clone(),
                cargo_home.clone(),
                Msg::CargoSubtree,
                Risk::Safe,
                true,
                true,
            ));
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Cargo,
        available,
        if available {
            None
        } else {
            Some(Msg::CargoNone.into())
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
        Category::Cargo,
        node.id,
        path,
        jail_root.clone(),
        Msg::CargoEntry,
        Risk::Safe,
        true,
        1,
    );
    send_nodes(tx, children);
}

fn resolve_cargo_home() -> PathBuf {
    std::env::var("CARGO_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".cargo")))
        .unwrap_or_else(|| PathBuf::from(".cargo"))
}
