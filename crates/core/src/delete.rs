use std::collections::HashSet;
use std::path::Path;
use std::thread;
use crossbeam_channel::{Receiver, Sender};

use crate::i18n::{Msg, Text};
use crate::model::{
    CompactProgress, DeleteEvent, DeleteOutcome, DeleteResultItem, DeleteStrategy, NodeTarget, ScanNode,
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

    // Virtual disks are compacted last, all together, so files deleted
    // inside WSL in this same run are already gone and only one
    // administrator prompt is shown.
    let (vhds, selected): (Vec<ScanNode>, Vec<ScanNode>) =
        selected.into_iter().partition(|n| n.is_vhd());

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
            NodeTarget::Vhd { .. } => unreachable!("virtual disks are handled below"),
        };

        let _ = tx.send(DeleteEvent::Item(result));
    }

    if !vhds.is_empty() {
        let _ = tx.send(DeleteEvent::Progress {
            done: selected.len(),
            total,
            current_label: Msg::VhdCompacting.into(),
        });
        for result in compact_vhds(&vhds, &tx) {
            let _ = tx.send(DeleteEvent::Item(result));
        }
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

/// Rough time to compact a disk of `bytes`, as (fastest, slowest) seconds.
///
/// diskpart reads the whole file, so the time grows with its size. The fast
/// end assumes an SSD; the slow end covers hard drives and heavily
/// fragmented disks. Shutting WSL down and attaching the disk add a little.
pub fn compact_estimate(bytes: u64) -> (u64, u64) {
    const MIB: u64 = 1024 * 1024;
    let fast = 15 + bytes / (400 * MIB);
    let slow = 60 + bytes / (40 * MIB);
    (fast, slow)
}

/// Sum of [`compact_estimate`] for several disks.
pub fn compact_estimate_all(sizes: impl IntoIterator<Item = u64>) -> (u64, u64) {
    sizes
        .into_iter()
        .map(compact_estimate)
        .fold((0, 0), |(a, b), (c, d)| (a + c, b + d))
}

/// Reads diskpart's output and returns (disks finished, percent of the
/// current one). diskpart prints lines such as "  45 percent completed"
/// (translated on non-English Windows, e.g. "45 por cento concluídos"), so
/// any number up to 100 on a line mentioning "cent" or "%" counts.
fn parse_diskpart_progress(log: &str) -> Option<(usize, u8)> {
    let mut finished = 0;
    let mut last = None;
    for seg in log.split(['\r', '\n']) {
        let lower = seg.to_lowercase();
        if !(lower.contains("cent") || lower.contains('%')) {
            continue;
        }
        let digits: String = seg
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(n) = digits.parse::<u8>() {
            if n <= 100 {
                if n == 100 && last != Some(100) {
                    finished += 1;
                }
                last = Some(n);
            }
        }
    }
    last.map(|l| if l == 100 { (finished, 0) } else { (finished, l) })
}

/// Compacts VHDX files with diskpart: shuts WSL down (which also stops
/// Docker Desktop), then runs one elevated diskpart script for every disk,
/// so Windows shows a single administrator prompt. Disks are attached
/// read-only; diskpart only gives back unused blocks. Progress is read
/// from diskpart's output while it runs.
fn compact_vhds(nodes: &[ScanNode], tx: &Sender<DeleteEvent>) -> Vec<DeleteResultItem> {
    let paths: Vec<&Path> = nodes
        .iter()
        .filter_map(|n| match &n.target {
            NodeTarget::Vhd { path } => Some(path.as_path()),
            _ => None,
        })
        .collect();

    if !cfg!(windows) {
        return nodes
            .iter()
            .map(|n| item(n, DeleteOutcome::Skipped, Some(Msg::VhdWindowsOnly.into()), 0))
            .collect();
    }

    let before: Vec<u64> = paths.iter().map(|p| crate::util::file_size(p)).collect();
    let estimate = compact_estimate_all(before.iter().copied());
    let started = std::time::Instant::now();
    let send = |disk: usize, percent: Option<u8>| {
        let disk = disk.min(nodes.len().saturating_sub(1));
        let _ = tx.send(DeleteEvent::Compacting(CompactProgress {
            disk,
            disks: nodes.len(),
            label: nodes[disk].label.clone(),
            elapsed_secs: started.elapsed().as_secs(),
            estimate_secs: estimate,
            percent,
        }));
    };
    send(0, None);

    let _ = crate::util::command("wsl").arg("--shutdown").output();
    // Give the WSL VM a moment to release the disk files.
    thread::sleep(std::time::Duration::from_secs(3));

    let mut script = String::new();
    for p in &paths {
        script.push_str(&format!(
            "select vdisk file=\"{}\"\r\nattach vdisk readonly\r\ncompact vdisk\r\ndetach vdisk\r\n",
            p.display()
        ));
    }
    script.push_str("exit\r\n");
    let tmp = std::env::temp_dir();
    let script_path = tmp.join(format!("devcleaner-compact-{}.txt", std::process::id()));
    let log_path = tmp.join(format!("devcleaner-compact-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log_path);

    // diskpart runs through cmd so its output goes to a file we can follow.
    // $ErrorActionPreference makes a refused administrator prompt fail.
    let mut ok = false;
    if std::fs::write(&script_path, script).is_ok() {
        let child = crate::util::command("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference = 'Stop'; \
                 $q = [char]34; \
                 $a = '/c diskpart /s ' + $q + $env:DEVCLEANER_SCRIPT + $q + ' > ' + $q + $env:DEVCLEANER_LOG + $q + ' 2>&1'; \
                 $p = Start-Process -FilePath 'cmd.exe' -ArgumentList $a -Verb RunAs -Wait -PassThru -WindowStyle Hidden; \
                 exit $p.ExitCode",
            ])
            .env("DEVCLEANER_SCRIPT", &script_path)
            .env("DEVCLEANER_LOG", &log_path)
            .spawn();
        if let Ok(mut child) = child {
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        ok = status.success();
                        break;
                    }
                    Ok(None) => {}
                    Err(_) => break,
                }
                let log = std::fs::read(&log_path)
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_default();
                match parse_diskpart_progress(&log) {
                    Some((done, pct)) => send(done, Some(pct)),
                    None => send(0, None),
                }
                thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_file(&log_path);

    nodes
        .iter()
        .zip(paths.iter().zip(before))
        .map(|(node, (path, before))| {
            let after = crate::util::file_size(path);
            let freed = before.saturating_sub(after);
            if !ok {
                item(node, DeleteOutcome::Failed, Some(Msg::VhdCompactFailed.into()), freed)
            } else if freed == 0 {
                item(node, DeleteOutcome::Skipped, Some(Msg::VhdNothingFreed.into()), 0)
            } else {
                item(node, DeleteOutcome::Removed, None, freed)
            }
        })
        .collect()
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
    crate::util::command(program)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_diskpart_progress_in_any_language() {
        assert_eq!(parse_diskpart_progress(""), None);
        let en = "Microsoft DiskPart\r\n  10 percent completed\r  45 percent completed\r";
        assert_eq!(parse_diskpart_progress(en), Some((0, 45)));
        let pt = "  100 por cento concluídos\r\nO DiskPart compactou o arquivo.\r\n  12 por cento concluídos\r";
        assert_eq!(parse_diskpart_progress(pt), Some((1, 12)));
        let done = "  100 percent completed\r\n";
        assert_eq!(parse_diskpart_progress(done), Some((1, 0)));
    }

    #[test]
    fn estimate_grows_with_size() {
        let gib = 1024 * 1024 * 1024;
        let (fast, slow) = compact_estimate(60 * gib);
        assert!(fast < slow);
        assert!(fast >= 60 && slow >= 20 * 60);
    }
}
