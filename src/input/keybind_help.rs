use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    config::{ActionKeybinds, IndexedKeybind, Keybinds},
    input::{mode_keys, TerminalKey},
};

pub(crate) type KeybindHelpEntry = (String, Cow<'static, str>);
pub(crate) type KeybindHelpGroup = (&'static str, Vec<KeybindHelpEntry>);

/// The character a key types, for overlay shortcuts and text fields.
pub(crate) fn keybind_help_text_char(key: &TerminalKey) -> Option<char> {
    if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
        return None;
    }
    // Text the host says the key produced is authoritative: kitty hosts may
    // report Shift+/ as `/` with text `?` and no shifted alternate.
    if let Some(text) = key.generated_text.as_deref() {
        let mut chars = text.chars();
        if let (Some(character), None) = (chars.next(), chars.next()) {
            if !character.is_control() {
                return Some(character);
            }
        }
    }
    if let Some(character) = key.shifted_codepoint.and_then(char::from_u32) {
        return Some(character);
    }
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    Some(character)
}

fn entry(key: impl Into<String>, label: &'static str) -> KeybindHelpEntry {
    (key.into(), Cow::Borrowed(label))
}

fn binding_label(bindings: &ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_owned())
}

fn indexed_label(bindings: &[IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return "unset".to_owned();
    }
    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }
    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

/// A key shown inside a mode: the key after the prefix, plus fixed alias keys.
fn mode_label(bindings: &ActionKeybinds, aliases: &[&str]) -> String {
    let mut labels = bindings.help_labels();
    for alias in aliases {
        if !labels.iter().any(|label| label == alias) {
            labels.push((*alias).to_owned());
        }
    }
    if labels.is_empty() {
        "unset".to_owned()
    } else {
        labels.join(" / ")
    }
}

fn indexed_mode_label(bindings: &[IndexedKeybind]) -> String {
    indexed_label(bindings)
        .split(" / ")
        .map(|part| part.strip_prefix("prefix+").unwrap_or(part))
        .collect::<Vec<_>>()
        .join(" / ")
}

pub(crate) fn keybind_help_groups(keybinds: &Keybinds) -> Vec<KeybindHelpGroup> {
    let mut groups = vec![
        (
            "global",
            vec![
                entry(binding_label(&keybinds.help), "keybinds"),
                entry(binding_label(&keybinds.settings), "settings"),
                entry(binding_label(&keybinds.goto), "session navigator"),
                entry(binding_label(&keybinds.new_worktree), "new worktree"),
                entry(binding_label(&keybinds.open_worktree), "open worktree"),
                entry(
                    binding_label(&keybinds.remove_worktree),
                    "delete worktree checkout",
                ),
                entry(binding_label(&keybinds.previous_agent), "previous agent"),
                entry(binding_label(&keybinds.next_agent), "next agent"),
                entry(indexed_label(&keybinds.focus_agent), "focus agent 1-9"),
                entry(binding_label(&keybinds.toggle_sidebar), "toggle sidebar"),
                entry(binding_label(&keybinds.detach), "detach"),
                entry(binding_label(&keybinds.reload_config), "reload config"),
                entry(
                    binding_label(&keybinds.open_notification_target),
                    "open notification target",
                ),
            ],
        ),
        (
            "modes",
            mode_keys::ALL
                .iter()
                .map(|mode| {
                    (
                        binding_label(mode.enter_binding(keybinds)),
                        Cow::Owned(format!("{} mode", mode.name)),
                    )
                })
                .collect(),
        ),
        (
            "tabs",
            vec![
                entry(mode_label(&keybinds.new_tab, &[]), "new tab"),
                entry(mode_label(&keybinds.rename_tab, &[]), "rename tab"),
                entry(
                    mode_label(&keybinds.previous_tab, &["h", "k", "←", "↑"]),
                    "previous tab",
                ),
                entry(
                    mode_label(&keybinds.next_tab, &["l", "j", "→", "↓"]),
                    "next tab",
                ),
                entry(
                    mode_label(&keybinds.move_tab_previous, &[]),
                    "move tab left",
                ),
                entry(mode_label(&keybinds.move_tab_next, &[]), "move tab right"),
                entry(indexed_mode_label(&keybinds.switch_tab), "switch tab 1-9"),
                entry(mode_label(&keybinds.close_tab, &[]), "close tab"),
            ],
        ),
        (
            "panes",
            vec![
                entry(mode_label(&keybinds.split_vertical, &[]), "split vertical"),
                entry(
                    mode_label(&keybinds.split_horizontal, &[]),
                    "split horizontal",
                ),
                entry(mode_label(&keybinds.close_pane, &[]), "close pane"),
                entry(mode_label(&keybinds.rename_pane, &[]), "rename pane"),
                entry(
                    mode_label(&keybinds.edit_scrollback, &[]),
                    "edit scrollback",
                ),
                entry(mode_label(&keybinds.clear_pane, &[]), "clear pane"),
                entry(mode_label(&keybinds.zoom, &[]), "zoom pane"),
                entry(mode_label(&keybinds.swap_pane_left, &[]), "swap pane left"),
                entry(mode_label(&keybinds.swap_pane_down, &[]), "swap pane down"),
                entry(mode_label(&keybinds.swap_pane_up, &[]), "swap pane up"),
                entry(
                    mode_label(&keybinds.swap_pane_right, &[]),
                    "swap pane right",
                ),
                entry(
                    binding_label(&keybinds.resize_pane_left),
                    "resize pane left",
                ),
                entry(
                    binding_label(&keybinds.resize_pane_down),
                    "resize pane down",
                ),
                entry(binding_label(&keybinds.resize_pane_up), "resize pane up"),
                entry(
                    binding_label(&keybinds.resize_pane_right),
                    "resize pane right",
                ),
                entry(
                    mode_label(&keybinds.focus_pane_left, &["←"]),
                    "focus pane left",
                ),
                entry(
                    mode_label(&keybinds.focus_pane_down, &["↓"]),
                    "focus pane down",
                ),
                entry(mode_label(&keybinds.focus_pane_up, &["↑"]), "focus pane up"),
                entry(
                    mode_label(&keybinds.focus_pane_right, &["→"]),
                    "focus pane right",
                ),
                entry(
                    mode_label(&keybinds.cycle_pane_next, &[]),
                    "cycle pane next",
                ),
                entry(
                    mode_label(&keybinds.cycle_pane_previous, &[]),
                    "cycle pane previous",
                ),
                entry(mode_label(&keybinds.last_pane, &[]), "last pane"),
            ],
        ),
    ];

    for mode in mode_keys::ALL {
        let rows = mode
            .help_rows(keybinds)
            .into_iter()
            .map(|(key, label)| (key, Cow::Borrowed(label)))
            .collect::<Vec<KeybindHelpEntry>>();
        let shared_group = match mode.name {
            "tab" => Some("tabs"),
            "pane" => Some("panes"),
            _ => None,
        };
        match shared_group {
            Some(name) => {
                if let Some((_, entries)) = groups.iter_mut().find(|(group, _)| *group == name) {
                    entries.extend(rows);
                }
            }
            None if !rows.is_empty() => groups.push((mode.name, rows)),
            None => {}
        }
    }

    if !keybinds.custom_commands.is_empty() {
        groups.push((
            "custom",
            keybinds
                .custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or(Cow::Borrowed("custom command")),
                    )
                })
                .collect(),
        ));
    }

    let mut unset = Vec::new();
    for (_, entries) in &mut groups {
        let (unbound, bound) = std::mem::take(entries)
            .into_iter()
            .partition::<Vec<_>, _>(|(key, _)| key == "unset");
        *entries = bound;
        unset.extend(unbound);
    }
    groups.retain(|(_, entries)| !entries.is_empty());
    if !unset.is_empty() {
        groups.push(("unset", unset));
    }
    groups
}

pub(crate) fn filter_keybind_help_groups(
    groups: Vec<KeybindHelpGroup>,
    query: &str,
) -> Vec<KeybindHelpGroup> {
    if query.is_empty() {
        return groups;
    }
    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|(key, label)| {
                    key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<KeybindHelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![entry("w", "workspace navigation"), entry("c", "new tab")],
            ),
            (
                "panes",
                vec![entry("v", "split vertical"), entry("x", "close pane")],
            ),
        ]
    }

    #[test]
    fn text_char_prefers_generated_text_then_alternate_then_code() {
        let parse = |bytes: &str| {
            crate::input::parse_terminal_key_sequence(bytes)
                .unwrap_or_else(|| panic!("{bytes:?} parses"))
        };
        for (bytes, want) in [
            // Shift+/ typing `?` with no shifted alternate.
            ("\x1b[47;2;63u", Some('?')),
            ("\x1b[47u", Some('/')),
            ("/", Some('/')),
            // Portuguese Shift+7: alternate and text agree.
            ("\x1b[55:47;2;47u", Some('/')),
            ("\x1b[55:47;2u", Some('/')),
            ("\x1b[55;2u", Some('7')),
            ("\x1b[47;5u", None),
            ("\x1b[47;3u", None),
        ] {
            assert_eq!(keybind_help_text_char(&parse(bytes)), want, "{bytes:?}");
        }
    }

    #[test]
    fn filter_matches_labels_and_shortcuts_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");

        let filtered = filter_keybind_help_groups(groups(), "x");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");
        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }

    #[test]
    fn default_help_omits_navigate_and_workspace_entries() {
        let groups = keybind_help_groups(&crate::config::Config::default().keybinds());
        assert!(groups.iter().all(|(group, _)| *group != "navigation"));
        let modes = groups
            .iter()
            .find_map(|(group, entries)| (*group == "modes").then_some(entries))
            .expect("modes group");
        let labels = modes
            .iter()
            .map(|(_, label)| label.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            [
                "pane mode",
                "tab mode",
                "session mode",
                "resize mode",
                "copy mode"
            ]
        );
        let mode_entries = groups
            .iter()
            .filter(|(group, _)| *group != "modes")
            .flat_map(|(_, entries)| entries)
            .filter(|(_, label)| label.ends_with(" mode") && label.as_ref() != "prefix mode");
        assert_eq!(mode_entries.count(), 0);
        for (_, entries) in &groups {
            for (_, label) in entries {
                assert!(!label.contains("workspace"), "{label}");
            }
        }
    }

    fn default_entries(group: &str) -> Vec<(String, String)> {
        keybind_help_groups(&crate::config::Config::default().keybinds())
            .into_iter()
            .find(|(name, _)| *name == group)
            .map(|(_, entries)| {
                entries
                    .into_iter()
                    .map(|(key, label)| (key, label.into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn has(group: &str, key: &str, label: &str) -> bool {
        default_entries(group)
            .iter()
            .any(|(k, l)| k == key && l == label)
    }

    #[test]
    fn groups_are_ordered_with_unset_keys_last() {
        let names = keybind_help_groups(&crate::config::Config::default().keybinds())
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["global", "modes", "tabs", "panes", "session", "resize", "copy", "unset"]
        );
    }

    #[test]
    fn only_global_and_mode_entry_keys_show_the_prefix() {
        assert!(has("global", "prefix+g", "session navigator"));
        assert!(has("global", "prefix+shift+g", "new worktree"));
        assert!(has("global", "prefix+b", "toggle sidebar"));
        assert!(has("modes", "prefix+p", "pane mode"));
        assert!(has("tabs", "c", "new tab"));
        assert!(has("tabs", "1..9", "switch tab 1-9"));
        assert!(has("panes", "v", "split vertical"));
        assert!(has("panes", "shift+h", "swap pane left"));
        assert!(!has("panes", "prefix+b", "toggle sidebar"));
    }

    #[test]
    fn fixed_alias_keys_are_merged_into_the_action_row() {
        assert!(has("tabs", "h / k / ← / ↑", "previous tab"));
        assert!(has("tabs", "n / l / j / → / ↓", "next tab"));
        assert!(has("panes", "h / ←", "focus pane left"));
        assert!(has("tabs", "esc / enter / t", "tab mode: leave"));
        assert!(has("panes", "esc / enter / p", "pane mode: leave"));
    }

    #[test]
    fn session_resize_and_copy_have_their_own_groups() {
        assert!(has("session", "s", "settings"));
        assert!(has("session", "esc / enter / shift+s", "leave"));
        assert!(has("resize", "h / ←", "resize pane left"));
        assert!(has("copy", "v / space", "select"));
        assert!(has(
            "copy",
            "PageUp / ctrl+b",
            "page up (ctrl+b is the prefix by default)"
        ));
    }

    #[test]
    fn unbound_actions_are_listed_under_unset() {
        let unset = default_entries("unset");
        for label in [
            "open worktree",
            "delete worktree checkout",
            "previous agent",
            "next agent",
            "focus agent 1-9",
            "move tab left",
            "move tab right",
            "clear pane",
            "resize pane left",
            "last pane",
        ] {
            assert!(
                unset.iter().any(|(k, l)| k == "unset" && l == label),
                "{label}"
            );
        }
        for group in ["global", "modes", "tabs", "panes"] {
            assert!(default_entries(group).iter().all(|(k, _)| k != "unset"));
        }
    }
}
