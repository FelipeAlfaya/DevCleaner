//! Helpers shared by the WSL and Docker scanners for virtual disks (VHDX).

use std::path::{Path, PathBuf};

use crate::i18n::{Msg, Text};
use crate::model::{Category, NodeTarget, Risk, ScanNode};
use crate::scanners::common::next_id;
use crate::util::{file_size, modified_time};

/// A virtual disk node. Selecting it compacts the file (it is never deleted),
/// so it starts unselected and as Review: compacting shuts WSL down.
pub fn vhd_node(category: Category, label: Text, path: PathBuf, reason: Msg) -> ScanNode {
    ScanNode {
        id: next_id(),
        parent_id: None,
        category,
        label,
        detail: path.display().to_string(),
        size_bytes: file_size(&path),
        modified: modified_time(&path),
        reason: reason.into(),
        risk: Risk::Review,
        default_selected: false,
        expandable: false,
        children_loaded: true,
        target: NodeTarget::Vhd { path },
    }
}

/// Every `.vhdx` file under `root`, up to `depth` levels down.
pub fn find_vhdx(root: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            if depth > 0 {
                out.extend(find_vhdx(&path, depth - 1));
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vhdx"))
        {
            out.push(path);
        }
    }
    out
}

/// Windows tools report `\\?\C:\...` paths; plain paths are easier to show
/// and to hand to diskpart.
pub fn strip_verbatim(p: &str) -> String {
    p.strip_prefix(r"\\?\").unwrap_or(p).to_string()
}

/// Decodes command output that may be UTF-16LE (wsl.exe) or UTF-8.
pub fn decode_output(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes.iter().skip(1).step_by(2).filter(|b| **b == 0).count() > bytes.len() / 4 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units).trim_start_matches('\u{feff}').to_string()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16_and_utf8() {
        let utf16: Vec<u8> = "Ubuntu\r\n".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decode_output(&utf16), "Ubuntu\r\n");
        assert_eq!(decode_output("Debian\n".as_bytes()), "Debian\n");
    }

    #[test]
    fn strips_verbatim_prefix() {
        assert_eq!(strip_verbatim(r"\\?\C:\wsl\Ubuntu"), r"C:\wsl\Ubuntu");
        assert_eq!(strip_verbatim(r"D:\x"), r"D:\x");
    }
}
