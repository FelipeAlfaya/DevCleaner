//! Code editors and AI apps built on Electron (VS Code and its forks such as
//! Cursor, Windsurf, Antigravity and Trae, plus the Claude desktop app) and
//! Claude Code.
//!
//! Only folders the apps rebuild on their own are marked Safe: browser and
//! GPU caches, compiled code caches, downloaded extension installers, logs
//! and crash reports. Per-workspace state (which in Cursor includes AI chat
//! history) and conversation transcripts are Review. Settings, installed
//! extensions, `globalStorage` and the local file history are never listed.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::i18n::{Msg, Text};
use crate::model::{Category, NodeTarget, Risk, ScanEvent, ScanNode};
use crate::scanners::common::{emit_category_summary, list_children, path_node, send_nodes};
use crate::settings::Settings;
use crate::util::{dir_size, home_dir};

/// (name shown, folder name under the OS config dir, updater folder on Windows)
const APPS: &[(&str, &str, Option<&str>)] = &[
    ("Cursor", "Cursor", Some("cursor-updater")),
    ("VS Code", "Code", None),
    ("VS Code Insiders", "Code - Insiders", None),
    ("VSCodium", "VSCodium", None),
    ("Windsurf", "Windsurf", Some("windsurf-updater")),
    ("Antigravity", "Antigravity", Some("antigravity-updater")),
    ("Antigravity IDE", "Antigravity IDE", None),
    ("Trae", "Trae", Some("trae-updater")),
    ("Kiro", "Kiro", Some("kiro-updater")),
    ("Claude", "Claude", Some("claude-updater")),
];

/// Folders Electron/VS Code recreate on demand.
const SAFE_DIRS: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "GrShaderCache",
    "ShaderCache",
    "CachedData",
    "CachedExtensionVSIXs",
    "CachedProfilesData",
    "logs",
    "Crashpad",
];

/// Entries smaller than this are left out to keep the list readable.
const MIN_BYTES: u64 = 1024 * 1024;

pub fn scan(_settings: &Settings, tx: &Sender<ScanEvent>, cancel: &AtomicBool) {
    let mut nodes = Vec::new();
    let config = directories::BaseDirs::new().map(|b| b.config_dir().to_path_buf());
    let local = crate::util::local_data_dir();

    for &(name, folder, updater) in APPS {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if let Some(base) = config.as_ref().map(|c| c.join(folder)) {
            if base.is_dir() {
                scan_app(name, &base, &mut nodes);
            }
        }
        if cfg!(windows) {
            if let (Some(local), Some(updater)) = (local.as_ref(), updater) {
                let dir = local.join(updater);
                push_if_big(
                    &mut nodes,
                    path_node(
                        Category::Ide,
                        None,
                        format!("{name} / {updater}"),
                        dir.clone(),
                        dir,
                        Msg::IdeUpdater,
                        Risk::Safe,
                        true,
                        false,
                    ),
                );
            }
        }
    }

    // Claude Code keeps one folder of conversation transcripts per project.
    if let Some(projects) = home_dir().map(|h| h.join(".claude").join("projects")) {
        if projects.is_dir() && dir_size(&projects) >= MIN_BYTES {
            nodes.push(path_node(
                Category::Ide,
                None,
                "Claude Code / projects",
                projects.clone(),
                projects,
                Msg::IdeClaudeCodeProjects,
                Risk::Review,
                false,
                true,
            ));
        }
    }

    let available = !nodes.is_empty();
    emit_category_summary(
        tx,
        Category::Ide,
        available,
        if available { None } else { Some(Msg::IdeNone.into()) },
        &nodes,
    );
    send_nodes(tx, nodes);
}

fn scan_app(name: &str, base: &Path, nodes: &mut Vec<ScanNode>) {
    for &dir in SAFE_DIRS {
        let path = base.join(dir);
        if path.is_dir() {
            push_if_big(
                nodes,
                path_node(
                    Category::Ide,
                    None,
                    format!("{name} / {dir}"),
                    path,
                    base.to_path_buf(),
                    Msg::IdeCache,
                    Risk::Safe,
                    true,
                    true,
                ),
            );
        }
    }

    let workspaces = base.join("User").join("workspaceStorage");
    if workspaces.is_dir() {
        push_if_big(
            nodes,
            path_node(
                Category::Ide,
                None,
                format!("{name} / workspaceStorage"),
                workspaces,
                base.to_path_buf(),
                Msg::IdeWorkspaceStorage,
                Risk::Review,
                false,
                true,
            ),
        );
    }

    // Cursor's local codebase index; rebuilt when a project is opened again.
    let snapshots = base.join("snapshots");
    if name == "Cursor" && snapshots.is_dir() {
        push_if_big(
            nodes,
            path_node(
                Category::Ide,
                None,
                format!("{name} / snapshots"),
                snapshots,
                base.to_path_buf(),
                Msg::IdeCursorIndex,
                Risk::Review,
                false,
                true,
            ),
        );
    }
}

fn push_if_big(nodes: &mut Vec<ScanNode>, node: ScanNode) {
    if node.size_bytes >= MIN_BYTES {
        nodes.push(node);
    }
}

pub fn expand(node: &ScanNode, _settings: &Settings, tx: &Sender<ScanEvent>) {
    let NodeTarget::Path { path, jail_root } = &node.target else {
        return;
    };
    let mut children = list_children(
        Category::Ide,
        node.id,
        path,
        jail_root.clone(),
        if node.risk == Risk::Review { Msg::IdeWorkspaceEntry } else { Msg::IdeCache },
        node.risk,
        node.risk == Risk::Safe,
        1,
    );
    // workspaceStorage folders are hashes; show the project they belong to.
    if path.file_name().is_some_and(|n| n == "workspaceStorage") {
        for child in &mut children {
            if let NodeTarget::Path { path, .. } = &child.target {
                if let Some(folder) = workspace_folder(path) {
                    child.label = Text::Raw(folder);
                }
            }
        }
    }
    send_nodes(tx, children);
}

/// Reads `workspace.json` and returns the project folder it points to.
fn workspace_folder(dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(dir.join("workspace.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let uri = json
        .get("folder")
        .or_else(|| json.get("workspace"))?
        .as_str()?;
    Some(uri_to_path(uri))
}

/// `file:///d%3A/projetos/app` -> `d:/projetos/app`; other schemes are kept
/// readable (e.g. `vscode-remote://wsl+ubuntu/home/me/app`).
fn uri_to_path(uri: &str) -> String {
    let rest = uri.strip_prefix("file:///").or_else(|| uri.strip_prefix("file://"));
    let decoded = percent_decode(rest.unwrap_or(uri));
    PathBuf::from(decoded).display().to_string()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_workspace_uris() {
        assert_eq!(percent_decode("d%3A/projetos/meu%20app"), "d:/projetos/meu app");
        assert_eq!(uri_to_path("file:///d%3A/projetos/cleaner"), "d:/projetos/cleaner");
        assert_eq!(percent_decode("100%"), "100%");
    }
}
