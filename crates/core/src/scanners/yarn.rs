use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::i18n::Msg;
use crate::model::{Category, Risk, ScanNode};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{home_dir, run_command, which};

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let cache = resolve_cache();
    let mut nodes = Vec::new();
    if !cache.exists() {
        emit_category_summary(
            tx,
            Category::Yarn,
            false,
            Some(Msg::YarnNone.into()),
            &nodes,
        );
        return;
    }

    let berry = is_berry();
    if berry {
        scan_berry(&cache, &mut nodes);
    } else {
        scan_classic(&cache, &mut nodes);
    }

    emit_category_summary(tx, Category::Yarn, true, None, &nodes);
    send_nodes(tx, nodes);
}

fn is_berry() -> bool {
    run_command("yarn", &["--version"], std::time::Duration::from_secs(5))
        .map(|v| {
            let major = v.split('.').next().unwrap_or("1");
            major.parse::<u32>().unwrap_or(1) >= 2
        })
        .unwrap_or(false)
}

fn scan_classic(cache: &PathBuf, nodes: &mut Vec<ScanNode>) {
    let jail = cache.clone();
    if let Ok(entries) = std::fs::read_dir(cache) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                nodes.push(path_node(
                    Category::Yarn,
                    None,
                    entry.file_name().to_string_lossy().to_string(),
                    path,
                    jail.clone(),
                    Msg::YarnClassic,
                    Risk::Safe,
                    true,
                    false,
                ));
            }
        }
    }
}

fn scan_berry(cache: &PathBuf, nodes: &mut Vec<ScanNode>) {
    let jail = cache.clone();
    if let Ok(entries) = std::fs::read_dir(cache) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "zip") {
                nodes.push(path_node(
                    Category::Yarn,
                    None,
                    entry.file_name().to_string_lossy().to_string(),
                    path,
                    jail.clone(),
                    Msg::YarnBerry,
                    Risk::Safe,
                    true,
                    false,
                ));
            }
        }
    }
}

fn resolve_cache() -> PathBuf {
    if which("yarn") {
        if let Some(out) = run_command("yarn", &["cache", "dir"], std::time::Duration::from_secs(5)) {
            return PathBuf::from(out.trim());
        }
    }
    home_dir()
        .map(|h| h.join(".cache").join("yarn"))
        .unwrap_or_else(|| PathBuf::from(".cache/yarn"))
}
