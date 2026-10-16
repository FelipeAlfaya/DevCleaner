use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::i18n::Msg;
use crate::model::{Category, DeleteStrategy};
use crate::scanners::common::{action_node, emit_category_summary, send_nodes};
use crate::settings::Settings;
use crate::util::{dir_size, home_dir, local_data_dir, run_command};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let store = resolve_store();
    let mut nodes = Vec::new();
    if store.exists() {
        let size = dir_size(&store);
        nodes.push(action_node(
            Category::Pnpm,
            Msg::PnpmLabel,
            store.display().to_string(),
            size,
            Msg::PnpmReason,
            DeleteStrategy::PnpmStorePrune,
            true,
        ));
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Pnpm,
        available,
        if available {
            None
        } else {
            Some(Msg::PnpmNone.into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn resolve_store() -> PathBuf {
    if let Some(out) = run_command("pnpm", &["store", "path"], std::time::Duration::from_secs(5)) {
        return PathBuf::from(out.trim());
    }
    if let Some(local) = local_data_dir() {
        let p = local.join("pnpm").join("store");
        if p.exists() {
            return p;
        }
    }
    home_dir()
        .map(|h| h.join(".local").join("share").join("pnpm").join("store"))
        .unwrap_or_else(|| PathBuf::from(".pnpm-store"))
}
