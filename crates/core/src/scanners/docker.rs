use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::i18n::{Msg, Text};
use crate::model::{
    Category, DeleteStrategy, NodeTarget, Risk, ScanNode,
};
use crate::scanners::common::{emit_category_summary, next_id, send_nodes};
use crate::settings::Settings;
use crate::util::which;

pub fn scan(settings: &Settings, tx: &crossbeam_channel::Sender<crate::model::ScanEvent>, cancel: &AtomicBool) {
    let _ = settings;
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    if !which("docker") || !docker_daemon_ok() {
        emit_category_summary(
            tx,
            Category::Docker,
            false,
            Some(Msg::DockerUnavailable.into()),
            &[],
        );
        return;
    }

    let mut nodes = Vec::new();
    nodes.extend(scan_images());
    nodes.extend(scan_containers());
    nodes.extend(scan_running_containers());
    nodes.extend(scan_volumes());
    nodes.extend(scan_build_cache());

    emit_category_summary(tx, Category::Docker, true, None, &nodes);
    send_nodes(tx, nodes);
}

fn docker_daemon_ok() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn scan_images() -> Vec<ScanNode> {
    let output = Command::new("docker")
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
    let output = Command::new("docker")
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
    let output = Command::new("docker")
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
    let output = Command::new("docker")
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
    let output = Command::new("docker")
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
