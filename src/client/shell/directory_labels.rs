use super::*;

pub(super) fn pane_directory(pane: &crate::protocol::ClientShellPane) -> Option<&str> {
    pane.foreground_cwd
        .as_deref()
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| pane.cwd.as_deref().filter(|cwd| !cwd.is_empty()))
}

pub(super) fn directory_name(path: &str) -> &str {
    let trimmed = trim_directory_separators(path);
    if trimmed.is_empty() {
        return path;
    }
    if is_windows_path(path) {
        trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
    } else {
        trimmed.rsplit('/').next().unwrap_or(trimmed)
    }
}

fn is_windows_path(path: &str) -> bool {
    // Endpoint paths may belong to another OS. Absolute Windows cwd values use
    // a drive prefix or UNC prefix; backslashes in Unix names are literal.
    let bytes = path.as_bytes();
    path.starts_with("\\\\")
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

fn trim_directory_separators(path: &str) -> &str {
    if is_windows_path(path) {
        path.trim_end_matches(['/', '\\'])
    } else {
        path.trim_end_matches('/')
    }
}

/// Build each label once, scanning the existing pane snapshot once rather than
/// scanning every pane for each tab. Pane snapshot order follows the layout.
/// `tabs` is the workspace's full tab order, before overflow scrolling.
pub(super) fn tab_labels(snapshot: &ClientShellSnapshot, tabs: &[&ClientShellTab]) -> Vec<String> {
    let mut labels = tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            if tab.custom_label {
                tab.label.clone()
            } else {
                format!("#{}", index + 1)
            }
        })
        .collect::<Vec<_>>();
    let mut auto_tabs = tabs
        .iter()
        .enumerate()
        .filter(|(_, tab)| !tab.custom_label)
        .map(|(index, tab)| {
            (
                (tab.workspace_id.as_str(), tab.tab_id.as_str()),
                (index, false),
            )
        })
        .collect::<HashMap<_, _>>();
    if !auto_tabs.is_empty() {
        let mut seen = HashSet::new();
        for pane in &snapshot.panes {
            let Some((index, has_directory)) =
                auto_tabs.get_mut(&(pane.workspace_id.as_str(), pane.tab_id.as_str()))
            else {
                continue;
            };
            let Some(directory) = pane_directory(pane) else {
                continue;
            };
            // Ignore trailing separators for duplicate detection, without any
            // filesystem access. Distinct full paths with the same basename stay distinct.
            if !seen.insert((*index, trim_directory_separators(directory))) {
                continue;
            }
            let label = &mut labels[*index];
            label.push_str(if *has_directory { " / " } else { " " });
            label.push_str(directory_name(directory));
            *has_directory = true;
        }
    }
    for (tab, label) in tabs.iter().zip(&mut labels) {
        if tab.zoomed {
            label.push_str(" Z");
        }
    }
    labels
}

pub(super) fn tab_directory_labels_changed(
    previous: &ClientShellSnapshot,
    next: &ClientShellSnapshot,
) -> bool {
    if previous.panes == next.panes {
        return false;
    }
    let active_labels = |snapshot: &ClientShellSnapshot| {
        let tabs = snapshot
            .tabs
            .iter()
            .filter(|tab| {
                Some(tab.workspace_id.as_str()) == snapshot.focused_workspace_id.as_deref()
            })
            .collect::<Vec<_>>();
        tab_labels(snapshot, &tabs)
    };
    active_labels(previous) != active_labels(next)
}
