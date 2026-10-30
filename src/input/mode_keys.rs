use crate::config::{ActionKeybinds, Keybinds};

type BindingOf = fn(&Keybinds) -> &ActionKeybinds;

pub(crate) enum ModeKeyLabel {
    Fixed(&'static str),
    Bound(BindingOf),
    /// The keys that leave a mode: these fixed keys, then the mode's own key.
    Leave(&'static str),
}

/// One key hint of a mode. `bar` is its text in the mode bar, `help` its text in
/// the keybinds help; a row is shown only where its text is set.
pub(crate) struct ModeKeyRow {
    label: ModeKeyLabel,
    bar: Option<&'static str>,
    help: Option<&'static str>,
}

pub(crate) struct ModeKeys {
    pub(crate) name: &'static str,
    enter: BindingOf,
    rows: &'static [ModeKeyRow],
}

const fn row(
    label: ModeKeyLabel,
    bar: Option<&'static str>,
    help: Option<&'static str>,
) -> ModeKeyRow {
    ModeKeyRow { label, bar, help }
}

const fn bar(label: ModeKeyLabel, text: &'static str) -> ModeKeyRow {
    row(label, Some(text), None)
}

const fn help(label: &'static str, text: &'static str) -> ModeKeyRow {
    row(ModeKeyLabel::Fixed(label), None, Some(text))
}

const fn leave(text: &'static str) -> ModeKeyRow {
    row(ModeKeyLabel::Leave("esc / enter"), None, Some(text))
}

const fn both(label: ModeKeyLabel, bar: &'static str, help: &'static str) -> ModeKeyRow {
    row(label, Some(bar), Some(help))
}

use ModeKeyLabel::{Bound, Fixed};

// The tab and pane mode keys that run an action are listed with the action in
// the keybinds help, so only their bar hints and the leave keys are here.
pub(crate) const PANE: ModeKeys = ModeKeys {
    name: "pane",
    enter: |kb| &kb.pane_mode,
    rows: &[
        bar(Fixed("h/j/k/l"), "move"),
        bar(Fixed("shift+h/j/k/l"), "swap"),
        bar(Bound(|kb| &kb.split_vertical), "split|"),
        bar(Bound(|kb| &kb.split_horizontal), "split-"),
        bar(Bound(|kb| &kb.close_pane), "close"),
        bar(Bound(|kb| &kb.zoom), "zoom"),
        bar(Bound(|kb| &kb.resize_mode), "resize"),
        leave("pane mode: leave"),
    ],
};

pub(crate) const TAB: ModeKeys = ModeKeys {
    name: "tab",
    enter: |kb| &kb.tab_mode,
    rows: &[
        bar(Fixed("h/l"), "switch"),
        bar(Bound(|kb| &kb.new_tab), "new"),
        bar(Bound(|kb| &kb.next_tab), "next"),
        bar(Fixed("1-9"), "jump"),
        bar(Bound(|kb| &kb.rename_tab), "rename"),
        bar(Bound(|kb| &kb.close_tab), "close"),
        leave("tab mode: leave"),
    ],
};

pub(crate) const SESSION: ModeKeys = ModeKeys {
    name: "session",
    enter: |kb| &kb.session_mode,
    rows: &[
        both(Bound(|kb| &kb.settings), "settings", "settings"),
        both(Bound(|kb| &kb.reload_config), "reload", "reload config"),
        both(Bound(|kb| &kb.help), "keybinds", "keybinds"),
        both(Bound(|kb| &kb.goto), "goto", "goto"),
        both(
            Bound(|kb| &kb.open_notification_target),
            "notify",
            "open notification target",
        ),
        both(Bound(|kb| &kb.toggle_sidebar), "sidebar", "toggle sidebar"),
        both(Bound(|kb| &kb.detach), "detach", "detach"),
        leave("leave"),
    ],
};

pub(crate) const RESIZE: ModeKeys = ModeKeys {
    name: "resize",
    enter: |kb| &kb.resize_mode,
    rows: &[
        bar(Fixed("h/l"), "width"),
        bar(Fixed("j/k"), "height"),
        help("h / ←", "resize pane left"),
        help("j / ↓", "resize pane down"),
        help("k / ↑", "resize pane up"),
        help("l / →", "resize pane right"),
        leave("leave"),
    ],
};

// The copy mode bar depends on search state, so only the help reads these rows.
pub(crate) const COPY: ModeKeys = ModeKeys {
    name: "copy",
    enter: |kb| &kb.copy_mode,
    rows: &[
        help("h / ←", "move cursor left"),
        help("j / ↓", "move cursor down"),
        help("k / ↑", "move cursor up"),
        help("l / →", "move cursor right"),
        help("w", "next word"),
        help("b", "previous word"),
        help("e", "word end"),
        help("W", "next big word"),
        help("B", "previous big word"),
        help("E", "big word end"),
        help("{", "previous paragraph"),
        help("}", "next paragraph"),
        help("0 / Home", "line start"),
        help("^", "first non-blank"),
        help("$ / End", "line end"),
        help("g", "top of history"),
        help("G", "bottom of history"),
        help(
            "PageUp / ctrl+b",
            "page up (ctrl+b is the prefix by default)",
        ),
        help("PageDown / ctrl+f", "page down"),
        help("ctrl+u", "half page up"),
        help("ctrl+d", "half page down"),
        help("v / space", "select"),
        help("V", "select lines"),
        help("/", "search forward"),
        help("?", "search backward"),
        help("n", "next match"),
        help("N", "previous match"),
        help("y / enter", "copy and leave"),
        help("esc", "clear selection or search, then leave"),
        help("q", "leave"),
        help("enter (while typing a search)", "run search"),
        help("esc (while typing a search)", "cancel search"),
    ],
};

/// In the order the prefix bar and the keybinds help list them.
pub(crate) const ALL: [&ModeKeys; 5] = [&PANE, &TAB, &SESSION, &RESIZE, &COPY];

impl ModeKeys {
    pub(crate) fn enter_binding<'a>(&self, keybinds: &'a Keybinds) -> &'a ActionKeybinds {
        (self.enter)(keybinds)
    }

    pub(crate) fn bar_hints(&self, keybinds: &Keybinds) -> Vec<(String, &'static str)> {
        self.hints(keybinds, |row| row.bar)
    }

    pub(crate) fn help_rows(&self, keybinds: &Keybinds) -> Vec<(String, &'static str)> {
        self.hints(keybinds, |row| row.help)
    }

    fn resolve(&self, label: &ModeKeyLabel, keybinds: &Keybinds) -> Option<String> {
        match label {
            Fixed(label) => Some((*label).to_owned()),
            Bound(binding) => binding(keybinds).prefix_rhs_label(),
            ModeKeyLabel::Leave(keys) => {
                Some(match self.enter_binding(keybinds).prefix_rhs_label() {
                    Some(own) => format!("{keys} / {own}"),
                    None => (*keys).to_owned(),
                })
            }
        }
    }

    fn hints(
        &self,
        keybinds: &Keybinds,
        text: fn(&ModeKeyRow) -> Option<&'static str>,
    ) -> Vec<(String, &'static str)> {
        self.rows
            .iter()
            .filter_map(|row| Some((self.resolve(&row.label, keybinds)?, text(row)?)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn rows_are_shown_in_the_bar_or_the_help() {
        for mode in ALL {
            assert!(!mode.rows.is_empty(), "{}", mode.name);
            for row in mode.rows {
                assert!(row.bar.is_some() || row.help.is_some(), "{}", mode.name);
            }
        }
    }

    #[test]
    fn bound_rows_follow_the_configured_keys() {
        let mut config = Config::default();
        config.keys.settings = crate::config::BindingConfig::one("prefix+x");
        config.keys.detach = crate::config::BindingConfig::empty();
        let keybinds = config.keybinds();

        let hints = SESSION.bar_hints(&keybinds);
        assert!(hints.contains(&("x".to_owned(), "settings")));
        assert!(hints.iter().all(|(_, text)| *text != "detach"));
    }

    #[test]
    fn leave_row_ends_with_the_key_that_enters_the_mode() {
        let keybinds = Config::default().keybinds();
        assert!(TAB
            .help_rows(&keybinds)
            .contains(&("esc / enter / t".to_owned(), "tab mode: leave")));
        assert!(SESSION
            .help_rows(&keybinds)
            .contains(&("esc / enter / shift+s".to_owned(), "leave")));
    }

    #[test]
    fn default_tab_bar_hints_keep_their_order() {
        let keybinds = Config::default().keybinds();
        let texts = TAB
            .bar_hints(&keybinds)
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>();
        assert_eq!(texts, ["switch", "new", "jump", "rename", "close"]);
    }
}
