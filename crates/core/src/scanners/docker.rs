use std::sync::atomic::{AtomicBool, Ordering};
use crate::i18n::{Msg, Text};
use crate::model::{
    Category, DeleteStrategy, NodeTarget, Risk, ScanNode,
};
use crate::scanners::common::{emit_category_summary, next_id, send_nodes};
use crate::settings::Settings;
use crate::util::which;
use crate::scanners::vhd::{find_vhdx, vhd_node};
use std::path::PathBuf;

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    // The virtual disk is read straight from the file system, so it shows up
    // even when Docker Desktop is closed.
    let mut nodes = scan_desktop_disks();

    if !which("docker") || !docker_daemon_ok() {
        let available = !nodes.is_empty();
        emit_category_summary(
            tx,
            Category::Docker,
            available,
            if available { None } else { Some(Msg::DockerUnavailable.into()) },
            &nodes,
        );
        send_nodes(tx, nodes);
        return;
    }

    nodes.extend(scan_images());
    nodes.extend(scan_unused_images());
    nodes.extend(scan_containers());
    nodes.extend(scan_running_containers());
    nodes.extend(scan_volumes());
    nodes.extend(scan_build_cache());

    emit_category_summary(tx, Category::Docker, true, None, &nodes);
    send_nodes(tx, nodes);
}

/// Folders where Docker Desktop keeps its WSL disks: the custom location set
/// in Settings > Resources > Advanced ("Disk image location"), if any, and
/// the default one.
fn desktop_disk_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(roaming) = directories::BaseDirs::new().map(|b| b.config_dir().to_path_buf()) {
        for file in ["settings-store.json", "settings.json"] {
            let Ok(raw) = std::fs::read_to_string(roaming.join("Docker").join(file)) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            if let Some(obj) = json.as_object() {
                for (k, v) in obj {
                    if k.eq_ignore_ascii_case("customWslDistroDir") {
                        if let Some(p) = v.as_str().filter(|p| !p.is_empty()) {
                            dirs.push(PathBuf::from(p));
                        }
                    }
                }
            }
        }
    }
    if let Some(local) = crate::util::local_data_dir() {
        dirs.push(local.join("Docker").join("wsl"));
    }
    dirs.dedup();
    dirs
}

fn scan_desktop_disks() -> Vec<ScanNode> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let mut seen = Vec::new();
    let mut nodes = Vec::new();
    for dir in desktop_disk_dirs() {
        for vhdx in find_vhdx(&dir, 3) {
            let key = vhdx.to_string_lossy().to_lowercase();
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let name = vhdx
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            nodes.push(vhd_node(
                Category::Docker,
                Text::with(Msg::DockerDiskLabel, name),
                vhdx,
                Msg::DockerDiskReason,
            ));
        }
    }
    nodes
}

/// Tagged images no container (running or stopped) was created from.
fn scan_unused_images() -> Vec<ScanNode> {
    let Some(images) = docker_lines(&[
        "image", "ls", "--no-trunc", "--filter", "dangling=false",
        "--format", "{{.ID}}|{{.Repository}}|{{.Tag}}|{{.Size}}",
    ]) else {
        return Vec::new();
    };
    let containers = docker_lines(&["container", "ls", "-a", "-q", "--no-trunc"]).unwrap_or_default();
    let used: Vec<String> = if containers.is_empty() {
        Vec::new()
    } else {
        let mut args = vec!["container", "inspect", "--format", "{{.Image}}"];
        args.extend(containers.iter().map(String::as_str));
        docker_lines(&args).unwrap_or_default()
    };

    images
        .iter()
        .filter_map(|line| {
            let mut parts = line.split('|');
            let id = parts.next()?.trim();
            let repo = parts.next().unwrap_or("").trim();
            let tag = parts.next().unwrap_or("").trim();
            let size = parse_docker_size(parts.next().unwrap_or("0").trim());
            if used.iter().any(|u| u.trim() == id) || repo == "<none>" {
                return None;
            }
            // Removing by name keeps other tags of the same image intact.
            let name = if tag.is_empty() || tag == "<none>" { repo.to_string() } else { format!("{repo}:{tag}") };
            Some(docker_node(
                &name,
                name.clone(),
                Msg::DockerUnusedImage,
                DeleteStrategy::DockerImage,
                Risk::Review,
                false,
                size,
            ))
        })
        .collect()
}

fn docker_lines(args: &[&str]) -> Option<Vec<String>> {
    let out = crate::util::command("docker").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .filter(|l| !l.trim().is_empty())
            .collect(),
    )
}

fn docker_daemon_ok() -> bool {
    crate::util::command("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn scan_images() -> Vec<ScanNode> {
    let output = crate::util::command("docker")
        .args(["images", "--filter", "dangling=true", "--format", "{{.ID}}|{{.Repository}}|{{.Tag}}|{{.Size}}"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_docker_lines(&String::from_utf8_lossy(&output.stdout), Msg::DockerImage, DeleteStrategy::DockerImage, Risk::Safe, true)
}

fn scan_containers() -> Vec<ScanNode> {
    let output = crate::util::command("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "status=exited",
            "--format",
            "{{.ID}}|{{.Names}}|exited|{{.Size}}",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    };
    parse_docker_lines(
        &String::from_utf8_lossy(&output.stdout),
        Msg::DockerContainer,
        DeleteStrategy::DockerContainer,
        // `docker rm` discards everything written inside the container that
        // is not in a volume (e.g. a dev database run without one).
        Risk::Review,
        false,
    )
}

fn scan_running_containers() -> Vec<ScanNode> {
    let output = crate::util::command("docker")
        .args([
            "ps",
            "--filter",
            "status=running",
            "--format",
            "{{.ID}}|{{.Names}}|running|{{.Size}}",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_docker_lines(
        &String::from_utf8_lossy(&output.stdout),
        Msg::DockerContainer,
        DeleteStrategy::DockerContainer,
        Risk::Blocked,
        false,
    )
}

fn scan_volumes() -> Vec<ScanNode> {
    let output = crate::util::command("docker")
        .args(["volume", "ls", "-qf", "dangling=true"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|id| docker_node(
            id.trim(),
            Text::with(Msg::DockerVolumeLabel, id),
            Msg::DockerVolumeReason,
            DeleteStrategy::DockerVolume,
            Risk::Review,
            false,
            0,
        ))
        .collect()
}

fn scan_build_cache() -> Vec<ScanNode> {
    let output = crate::util::command("docker")
        .args(["buildx", "du", "--format", "{{.ID}}|{{.Size}}"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split('|');
            let id = parts.next()?.trim();
            let size_str = parts.next().unwrap_or("0").trim();
            let size = parse_docker_size(size_str);
            Some(docker_node(
                id,
                Text::with(Msg::DockerBuildCacheLabel, id),
                Msg::DockerBuildCacheReason,
                DeleteStrategy::DockerBuildCache,
                Risk::Safe,
                true,
                size,
            ))
        })
        .collect()
}

fn parse_docker_lines(
    text: &str,
    reason: Msg,
    strategy: DeleteStrategy,
    risk: Risk,
    default_selected: bool,
) -> Vec<ScanNode> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split('|');
            let id = parts.next()?.trim();
            let name = parts.next().unwrap_or("").trim();
            let tag = parts.next().unwrap_or("").trim();
            let size_str = parts.next().unwrap_or("0").trim();
            let label = if name.is_empty() {
                id.to_string()
            } else if tag.is_empty() {
                name.to_string()
            } else {
                format!("{name}:{tag}")
            };
            Some(docker_node(
                id,
                label,
                reason,
                strategy,
                risk,
                default_selected,
                parse_docker_size(size_str),
            ))
        })
        .collect()
}

fn docker_node(
    id: &str,
    label: impl Into<Text>,
    reason: Msg,
    strategy: DeleteStrategy,
    risk: Risk,
    default_selected: bool,
    size_bytes: u64,
) -> ScanNode {
    ScanNode {
        id: next_id(),
        parent_id: None,
        category: Category::Docker,
        label: label.into(),
        detail: id.to_string(),
        size_bytes,
        modified: None,
        reason: reason.into(),
        risk,
        default_selected,
        expandable: false,
        children_loaded: true,
        target: NodeTarget::DockerId {
            id: id.to_string(),
            strategy,
        },
    }
}

fn parse_docker_size(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return 0;
    }
    let value: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();
    let mult = match unit.as_str() {
        "B" => 1.0,
        "KB" | "KIB" => 1024.0,
        "MB" | "MIB" => 1024.0 * 1024.0,
        "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (value * mult) as u64
}
