use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::thread;
use crossbeam_channel::{Receiver, Sender};

use crate::i18n::{Msg, Text};
use crate::model::{
    DeleteEvent, DeleteOutcome, DeleteResultItem, DeleteStrategy, NodeTarget, ScanNode,
};
use crate::safety::validate_delete_path;

pub struct DeleteHandle {
    pub events: Receiver<DeleteEvent>,
}

pub fn start_delete(nodes: Vec<ScanNode>, selected_ids: HashSet<u64>) -> DeleteHandle {
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::spawn(move || run_delete(nodes, selected_ids, tx));
    DeleteHandle { events: rx }
}

fn run_delete(nodes: Vec<ScanNode>, selected_ids: HashSet<u64>, tx: Sender<DeleteEvent>) {
    let selected: Vec<ScanNode> = nodes
        .into_iter()
        .filter(|n| selected_ids.contains(&n.id))
        .collect();
    let total = selected.len();

    let mut ran_category: HashSet<DeleteStrategy> = HashSet::new();

    for (idx, node) in selected.iter().enumerate() {
        let _ = tx.send(DeleteEvent::Progress {
            done: idx,
            total,
            current_label: node.label.clone(),
        });

        let result = match &node.target {
            NodeTarget::CategoryAction { strategy, .. } => {
                if ran_category.contains(strategy) {
                    DeleteResultItem {
                        node_id: node.id,
                        label: node.label.clone(),
                        outcome: DeleteOutcome::Skipped,
                        message: Some(Msg::DeleteAlreadyRun.into()),
                        bytes_freed: 0,
                    }
                } else {
                    ran_category.insert(*strategy);
                    run_category_action(*strategy, node)
                }
            }
            NodeTarget::DockerId { id, strategy } => run_docker(id, *strategy, node),
            NodeTarget::Path { path, jail_root } => delete_path(node, path, jail_root, &selected),
        };

        let _ = tx.send(DeleteEvent::Item(result));
    }

    let _ = tx.send(DeleteEvent::Progress {
        done: total,
        total,
        current_label: Text::Raw(String::new()),
    });
    let _ = tx.send(DeleteEvent::Finished);
}

fn delete_path(
    node: &ScanNode,
    path: &Path,
    jail_root: &Path,
    selected: &[ScanNode],
) -> DeleteResultItem {
    if node.risk == crate::model::Risk::Blocked {
        return item(node, DeleteOutcome::Skipped, Some(Msg::DeleteBlocked.into()), 0);
    }

    match validate_delete_path(jail_root, path) {
        Err(e) => item(node, DeleteOutcome::Failed, Some(e.msg().into()), 0),
        Ok(safe_path) => {
            let size_before = node.size_bytes;
            let has_selected_child = selected.iter().any(|n| n.parent_id == Some(node.id));
            let result = if safe_path.is_dir() && has_selected_child {
                remove_files_only(&safe_path, selected, node.id)
            } else if safe_path.is_dir() {
                std::fs::remove_dir_all(&safe_path)
            } else {
                std::fs::remove_file(&safe_path)
            };

            match result {
                Ok(()) => item(node, DeleteOutcome::Removed, None, size_before),
                Err(e) => item(node, DeleteOutcome::Failed, Some(Text::Raw(e.to_string())), 0),
            }
        }
    }
}

fn remove_files_only(
    dir: &Path,
    selected: &[ScanNode],
    parent_id: u64,
) -> std::io::Result<()> {
    for child in selected.iter().filter(|n| n.parent_id == Some(parent_id)) {
        if let NodeTarget::Path { path, jail_root } = &child.target {
            let safe = validate_delete_path(jail_root, path).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "outside the allowed root")
            })?;
            if safe.is_dir() {
                std::fs::remove_dir_all(safe)?;
            } else {
                std::fs::remove_file(safe)?;
            }
        }
    }
    let _ = dir;
    Ok(())
}

fn run_category_action(strategy: DeleteStrategy, node: &ScanNode) -> DeleteResultItem {
    let ok = match strategy {
        DeleteStrategy::NpmCacheClean => run_cmd("npm", &["cache", "clean", "--force"]),
        DeleteStrategy::PnpmStorePrune => run_cmd("pnpm", &["store", "prune"]),
        DeleteStrategy::GoCleanCache => run_cmd("go", &["clean", "-cache"]),
        DeleteStrategy::GoCleanModCache => run_cmd("go", &["clean", "-modcache"]),
        _ => false,
    };
    if ok {
        item(node, DeleteOutcome::Removed, None, node.size_bytes)
    } else {
        item(
            node,
            DeleteOutcome::Failed,
            Some(Msg::DeleteToolFailed.into()),
            0,
        )
    }
}

fn run_docker(id: &str, strategy: DeleteStrategy, node: &ScanNode) -> DeleteResultItem {
    if node.risk == crate::model::Risk::Blocked {
        return item(node, DeleteOutcome::Skipped, Some(Msg::DeleteBlocked.into()), 0);
    }
    let ok = match strategy {
        DeleteStrategy::DockerImage => run_cmd("docker", &["rmi", id]),
        DeleteStrategy::DockerContainer => run_cmd("docker", &["rm", id]),
        DeleteStrategy::DockerVolume => run_cmd("docker", &["volume", "rm", id]),
        DeleteStrategy::DockerBuildCache => run_cmd("docker", &["buildx", "prune", "-f", "--filter", &format!("id={id}")]),
        _ => false,
    };
    if ok {
        item(node, DeleteOutcome::Removed, None, node.size_bytes)
    } else {
        item(
            node,
            DeleteOutcome::Failed,
            Some(Msg::DeleteDockerFailed.into()),
            0,
        )
    }
}

fn run_cmd(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn item(
    node: &ScanNode,
    outcome: DeleteOutcome,
    message: Option<Text>,
    bytes_freed: u64,
) -> DeleteResultItem {
    DeleteResultItem {
        node_id: node.id,
        label: node.label.clone(),
        outcome,
        message,
        bytes_freed,
    }
}
