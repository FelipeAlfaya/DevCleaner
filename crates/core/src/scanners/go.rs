use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::i18n::Msg;
use crate::model::{Category, DeleteStrategy};
use crate::scanners::common::{action_node, emit_category_summary, send_nodes};
use crate::settings::Settings;
use crate::util::{dir_size, home_dir, run_command};

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
                Msg::GoBuildLabel,
                build_cache.display().to_string(),
                size,
                Msg::GoBuildReason,
                DeleteStrategy::GoCleanCache,
                true,
            ));
        }
    }

    if let Some(mod_cache) = resolve_mod_cache() {
        if mod_cache.exists() {
            // Go writes the module cache read-only, so deleting it file by file
            // fails (and can leave it half-removed). `go clean -modcache` is the
            // supported way to clear it.
            let size = dir_size(&mod_cache);
            nodes.push(action_node(
                Category::Go,
                Msg::GoModLabel,
                mod_cache.display().to_string(),
                size,
                Msg::GoModReason,
                DeleteStrategy::GoCleanModCache,
                true,
            ));
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
            Some(Msg::GoNone.into())
        },
        &nodes,
    );
    send_nodes(tx, nodes);
}

/// Asks the Go toolchain first (`go env`), which knows the real location on
/// every OS (e.g. %LocalAppData%\go-build on Windows), then falls back to the
/// environment and the usual defaults.
fn go_env(var: &str) -> Option<PathBuf> {
    run_command("go", &["env", var], std::time::Duration::from_secs(5))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "off")
        .map(PathBuf::from)
}

fn resolve_build_cache() -> Option<PathBuf> {
    go_env("GOCACHE")
        .or_else(|| std::env::var("GOCACHE").ok().map(PathBuf::from))
        .or_else(|| {
            if cfg!(windows) {
                crate::util::local_data_dir().map(|l| l.join("go-build"))
            } else {
                home_dir().map(|h| h.join(".cache").join("go-build"))
            }
        })
}

fn resolve_mod_cache() -> Option<PathBuf> {
    go_env("GOMODCACHE")
        .or_else(|| std::env::var("GOMODCACHE").ok().map(PathBuf::from))
        .or_else(|| {
            // GOPATH can hold several entries; the module cache lives in the first.
            std::env::var_os("GOPATH")
                .and_then(|g| std::env::split_paths(&g).next())
                .map(|g| g.join("pkg").join("mod"))
        })
        .or_else(|| home_dir().map(|h| h.join("go").join("pkg").join("mod")))
}
