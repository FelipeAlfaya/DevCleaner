use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{Category, DeleteStrategy};
use crate::scanners::common::{action_node, emit_category_summary, send_nodes};
use crate::settings::Settings;
use crate::util::{dir_size, home_dir, local_data_dir, run_command};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let cache = resolve_cache();
    let mut nodes = Vec::new();
    if cache.exists() {
        let size = dir_size(&cache);
        nodes.push(action_node(
            Category::Npm,
            "Cache npm (_cacache)".into(),
            cache.display().to_string(),
            size,
            "Limpeza via npm cache clean --force (não é seguro apagar arquivos isolados)",
            DeleteStrategy::NpmCacheClean,
            true,
        ));
    }

    let available = !nodes.is_empty();
    let reason = if available {
        None
    } else {
        Some("Cache npm não encontrado".into())
    };
    emit_category_summary(tx, Category::Npm, available, reason, &nodes);
    send_nodes(tx, nodes);
}

fn resolve_cache() -> PathBuf {
    if let Some(out) = run_command("npm", &["config", "get", "cache"], std::time::Duration::from_secs(5)) {
        let p = PathBuf::from(out.trim());
        if p.exists() {
            return p;
        }
    }
    if let Some(local) = local_data_dir() {
        let p = local.join("npm-cache");
        if p.exists() {
            return p;
        }
    }
    home_dir().map(|h| h.join(".npm")).unwrap_or_else(|| PathBuf::from(".npm"))
}
