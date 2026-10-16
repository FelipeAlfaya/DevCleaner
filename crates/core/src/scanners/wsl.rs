//! WSL (Windows Subsystem for Linux).
//!
//! Each distro lives in a virtual disk (`ext4.vhdx`) that grows as files are
//! written inside Linux but never shrinks on its own. This scanner lists those
//! disks, so they can be compacted, and the usual dev caches inside the home
//! folders of distros that are already running (reached through
//! `\\wsl.localhost\<distro>`). Stopped distros are not started by a scan.
//!
//! Docker Desktop's own distros are handled by the Docker scanner.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::i18n::{Msg, Text};
use crate::model::{Category, Risk, ScanEvent, ScanNode};
use crate::scanners::common::{emit_category_summary, path_node, send_nodes};
use crate::scanners::vhd::{decode_output, strip_verbatim, vhd_node};
use crate::settings::Settings;
use crate::util::command;

pub struct Distro {
    pub name: String,
    pub vhdx: Option<PathBuf>,
}

/// Dev caches looked for inside each Linux home folder.
/// (path relative to home, risk)
const LINUX_CACHES: &[(&str, Risk)] = &[
    (".cache/pip", Risk::Safe),
    (".cache/uv", Risk::Safe),
    (".cache/go-build", Risk::Safe),
    (".cache/yarn", Risk::Safe),
    (".cache/pnpm", Risk::Safe),
    (".npm/_cacache", Risk::Safe),
    (".bun/install/cache", Risk::Safe),
    (".cargo/registry", Risk::Safe),
    (".gradle/caches", Risk::Safe),
    (".m2/repository", Risk::Review),
    (".vscode-server/data/CachedExtensionVSIXs", Risk::Safe),
    (".cursor-server/data/CachedExtensionVSIXs", Risk::Safe),
];

pub fn scan(_settings: &Settings, tx: &Sender<ScanEvent>, cancel: &AtomicBool) {
    if !cfg!(windows) {
        emit_category_summary(tx, Category::Wsl, false, Some(Msg::WslWindowsOnly.into()), &[]);
        return;
    }

    let distros: Vec<Distro> = list_distros()
        .into_iter()
        .filter(|d| !is_docker_distro(&d.name))
        .collect();
    if distros.is_empty() {
        emit_category_summary(tx, Category::Wsl, false, Some(Msg::WslNone.into()), &[]);
        return;
    }
    let running = running_distros();

    let mut nodes = Vec::new();
    for d in &distros {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if let Some(vhdx) = d.vhdx.as_ref().filter(|p| p.is_file()) {
            nodes.push(vhd_node(
                Category::Wsl,
                Text::with(Msg::WslDiskLabel, d.name.clone()),
                vhdx.clone(),
                Msg::WslDiskReason,
            ));
        }
        if running.iter().any(|r| r.eq_ignore_ascii_case(&d.name)) {
            nodes.extend(scan_distro_caches(&d.name));
        }
    }

    emit_category_summary(tx, Category::Wsl, true, None, &nodes);
    send_nodes(tx, nodes);
}

pub fn is_docker_distro(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("docker-desktop")
}

/// Registered distros, read from the registry (where WSL itself keeps them).
pub fn list_distros() -> Vec<Distro> {
    const SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
        $k='HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss'; \
        if (Test-Path $k) { Get-ChildItem $k | ForEach-Object { \
        $p = Get-ItemProperty $_.PSPath; \
        \"$($p.DistributionName)|$($p.BasePath)|$($p.VhdFileName)\" } }";
    let Ok(out) = command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .output()
    else {
        return Vec::new();
    };
    parse_distros(&String::from_utf8_lossy(&out.stdout))
}

fn parse_distros(text: &str) -> Vec<Distro> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.trim().split('|');
            let name = parts.next()?.trim();
            if name.is_empty() {
                return None;
            }
            let base = strip_verbatim(parts.next().unwrap_or("").trim());
            let file = parts.next().map(str::trim).filter(|f| !f.is_empty()).unwrap_or("ext4.vhdx");
            let vhdx = (!base.is_empty()).then(|| PathBuf::from(base).join(file));
            Some(Distro { name: name.to_string(), vhdx })
        })
        .collect()
}

/// Names of distros running right now.
pub fn running_distros() -> Vec<String> {
    let Ok(out) = command("wsl")
        .args(["--list", "--running", "--quiet"])
        .env("WSL_UTF8", "1")
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    decode_output(&out.stdout)
        .lines()
        .map(|l| l.trim().trim_matches('\0').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// `\\wsl.localhost\<distro>` on current Windows, `\\wsl$\<distro>` on older.
fn distro_root(name: &str) -> Option<PathBuf> {
    [format!(r"\\wsl.localhost\{name}"), format!(r"\\wsl$\{name}")]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.join("home").is_dir() || p.join("root").is_dir())
}

fn home_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    let mut homes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root.join("home")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && !p.is_symlink() {
                homes.push((format!("~{}", e.file_name().to_string_lossy()), p));
            }
        }
    }
    let r = root.join("root");
    if r.is_dir() {
        homes.push(("/root".to_string(), r));
    }
    homes
}

fn scan_distro_caches(name: &str) -> Vec<ScanNode> {
    let Some(root) = distro_root(name) else {
        return Vec::new();
    };
    let mut nodes = Vec::new();
    for (who, home) in home_dirs(&root) {
        for &(rel, risk) in LINUX_CACHES {
            let path = rel.split('/').fold(home.clone(), |p, part| p.join(part));
            if !path.is_dir() || path.is_symlink() {
                continue;
            }
            let node = path_node(
                Category::Wsl,
                None,
                format!("{name}: {who}/{rel}"),
                path,
                home.clone(),
                Msg::WslCache,
                risk,
                risk == Risk::Safe,
                true,
            );
            if node.size_bytes > 0 {
                nodes.push(node);
            }
        }
    }
    nodes
}

pub fn expand(node: &ScanNode, _settings: &Settings, tx: &Sender<ScanEvent>) {
    let crate::model::NodeTarget::Path { path, jail_root } = &node.target else {
        return;
    };
    let children = crate::scanners::common::list_children(
        Category::Wsl,
        node.id,
        path,
        jail_root.clone(),
        Msg::WslCache,
        node.risk,
        node.risk == Risk::Safe,
        1,
    );
    send_nodes(tx, children);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_lines() {
        let out = "Ubuntu|C:\\Users\\me\\AppData\\Local\\Packages\\Canonical\\LocalState|\r\n\
                   docker-desktop|\\\\?\\D:\\DockerDesktopWSL\\main|ext4.vhdx\r\n\
                   Arch|D:\\wsl\\arch|disk.vhdx\r\n\r\n";
        let d = parse_distros(out);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].name, "Ubuntu");
        assert!(d[0].vhdx.as_ref().unwrap().ends_with("ext4.vhdx"));
        assert!(d[1].vhdx.as_ref().unwrap().to_string_lossy().starts_with("D:"));
        assert!(d[2].vhdx.as_ref().unwrap().ends_with("disk.vhdx"));
        assert!(is_docker_distro(&d[1].name));
    }
}
