use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::Sender;

use crate::model::{
    Category, CategoryStatus, DeleteStrategy, NodeTarget, Risk, ScanEvent, ScanNode,
};
use crate::util::{dir_size, file_size, modified_time};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn send_status(tx: &Sender<ScanEvent>, status: CategoryStatus) {
    let _ = tx.send(ScanEvent::CategoryStatus(status));
}

pub fn send_nodes(tx: &Sender<ScanEvent>, nodes: Vec<ScanNode>) {
    if nodes.is_empty() {
        return;
    }
    let _ = tx.send(ScanEvent::NodesBatch(nodes));
}

pub fn path_node(
    category: Category,
    parent_id: Option<u64>,
    label: String,
    path: PathBuf,
    jail_root: PathBuf,
    reason: &str,
    risk: Risk,
    default_selected: bool,
    expandable: bool,
) -> ScanNode {
    let size_bytes = if path.is_dir() {
        dir_size(&path)
    } else {
        file_size(&path)
    };
    ScanNode {
        id: next_id(),
        parent_id,
        category,
        label,
        detail: path.display().to_string(),
        size_bytes,
        modified: modified_time(&path),
        reason: reason.to_string(),
        risk,
        default_selected,
        expandable,
        children_loaded: !expandable,
        target: NodeTarget::Path { path, jail_root },
    }
}

pub fn action_node(
    category: Category,
    label: String,
    detail: String,
    size_bytes: u64,
    reason: &str,
    strategy: DeleteStrategy,
    default_selected: bool,
) -> ScanNode {
    ScanNode {
        id: next_id(),
        parent_id: None,
        category,
        label,
        detail,
        size_bytes,
        modified: None,
        reason: reason.to_string(),
        risk: Risk::Safe,
        default_selected,
        expandable: false,
        children_loaded: true,
        target: NodeTarget::CategoryAction {
            category,
            strategy,
        },
    }
}

pub fn list_children(
    category: Category,
    parent_id: u64,
    root: &Path,
    jail_root: PathBuf,
    reason: &str,
    risk: Risk,
    default_selected: bool,
    max_depth: usize,
) -> Vec<ScanNode> {
    let mut nodes = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return nodes,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let expandable = path.is_dir() && max_depth > 1;
        nodes.push(path_node(
            category,
            Some(parent_id),
            name,
            path,
            jail_root.clone(),
            reason,
            risk,
            default_selected,
            expandable,
        ));
    }
    nodes
}

pub fn emit_category_summary(
    tx: &Sender<ScanEvent>,
    category: Category,
    available: bool,
    reason: Option<String>,
    nodes: &[ScanNode],
) {
    let total_bytes: u64 = nodes.iter().map(|n| n.size_bytes).sum();
    send_status(
        tx,
        CategoryStatus {
            category,
            available,
            unavailable_reason: reason,
            total_bytes,
            item_count: nodes.len(),
        },
    );
}
