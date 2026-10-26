use super::*;

pub(super) fn pane_directory(pane: &crate::protocol::ClientShellPane) -> Option<&str> {
    pane.foreground_cwd
        .as_deref()
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| pane.cwd.as_deref().filter(|cwd| !cwd.is_empty()))
}

pub(super) fn directory_name(path: &str) -> &str {
    // Endpoint paths may be Windows paths even when the TUI runs on Unix.
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return path;
    }
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
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
            if !seen.insert((*index, directory.trim_end_matches(['/', '\\']))) {
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
