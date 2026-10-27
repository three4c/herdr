use super::*;

fn labels(snapshot: &ClientShellSnapshot) -> Vec<String> {
    super::super::directory_labels::tab_labels(snapshot, &snapshot.tabs.iter().collect::<Vec<_>>())
}

fn add_pane(snapshot: &mut ClientShellSnapshot, id: &str, directory: Option<&str>) {
    let mut pane = snapshot.panes[0].clone();
    pane.pane_id = id.into();
    pane.cwd = directory.map(str::to_string);
    pane.foreground_cwd = None;
    pane.focused = false;
    snapshot.panes.push(pane);
}

#[test]
fn automatic_tab_labels_use_hash_numbers_and_available_directories() {
    let mut projected = snapshot();
    projected.panes.clear();
    assert_eq!(labels(&projected), ["#1"]);
    projected = snapshot();
    assert_eq!(labels(&projected), ["#1 repo"]);
    projected.panes[0].foreground_cwd = Some("/Users/test/Develop/herdr".into());
    assert_eq!(labels(&projected), ["#1 herdr"]);
    projected.panes[0].foreground_cwd = Some(String::new());
    projected.panes[0].cwd = Some("/Users/test/Develop/dotfiles".into());
    assert_eq!(labels(&projected), ["#1 dotfiles"]);
    projected.panes[0].cwd = None;
    assert_eq!(labels(&projected), ["#1"]);
}

#[test]
fn split_tab_labels_follow_layout_order_and_deduplicate_full_directories() {
    let mut projected = snapshot();
    projected.panes[0].cwd = Some("/work/herdr".into());
    projected.panes[0].foreground_cwd = None;
    add_pane(&mut projected, "pane_hoge", Some("/work/hoge"));
    add_pane(&mut projected, "pane_herdr_again", Some("/work/herdr/"));
    add_pane(&mut projected, "pane_missing", None);
    assert_eq!(labels(&projected), ["#1 herdr / hoge"]);
    projected.panes[1].focused = true;
    projected.panes[0].focused = false;
    assert_eq!(labels(&projected), ["#1 herdr / hoge"]);

    // The original pane follows cd rather than retaining its initial label.
    projected.panes[0].cwd = Some("/work/dotfiles".into());
    assert_eq!(labels(&projected), ["#1 dotfiles / hoge / herdr"]);
    projected.panes.remove(2);
    assert_eq!(labels(&projected), ["#1 dotfiles / hoge"]);
    projected.panes.swap(0, 1);
    assert_eq!(labels(&projected), ["#1 hoge / dotfiles"]);
}

#[test]
fn distinct_directories_with_the_same_basename_remain_distinct() {
    let mut projected = snapshot();
    projected.panes[0].cwd = Some("/work/herdr".into());
    projected.panes[0].foreground_cwd = None;
    add_pane(&mut projected, "pane_other", Some("/other/herdr"));
    assert_eq!(labels(&projected), ["#1 herdr / herdr"]);
}

#[test]
fn directory_tab_labels_use_positions_and_preserve_manual_names_zoom_and_ids() {
    let mut projected = snapshot();
    projected.tabs[0].number = 7;
    projected.tabs[0].tab_id = "tab_7".into();
    projected.panes[0].tab_id = "tab_7".into();
    projected.tabs[0].zoomed = true;
    assert_eq!(labels(&projected), ["#1 repo Z"]);
    assert_eq!(projected.tabs[0].number, 7);
    assert_eq!(projected.tabs[0].tab_id, "tab_7");
    projected.tabs[0].custom_label = true;
    projected.tabs[0].label = "review".into();
    assert_eq!(labels(&projected), ["review Z"]);
    projected.tabs[0].label = "1".into();
    projected.tabs[0].zoomed = false;
    assert_eq!(labels(&projected), ["1"]);
}

#[test]
fn directory_tab_labels_group_panes_by_workspace_and_tab() {
    let mut projected = snapshot();
    let mut second = projected.tabs[0].clone();
    second.tab_id = "tab_2".into();
    second.number = 2;
    projected.tabs.push(second);
    add_pane(&mut projected, "pane_2", Some("/other/hoge"));
    projected.panes[1].tab_id = "tab_2".into();
    add_pane(&mut projected, "pane_foreign", Some("/foreign/ignored"));
    projected.panes[2].workspace_id = "ws_other".into();
    assert_eq!(labels(&projected), ["#1 repo", "#2 hoge"]);
    projected.tabs.swap(0, 1);
    assert_eq!(labels(&projected), ["#1 hoge", "#2 repo"]);
}

#[test]
fn directory_tab_labels_handle_remote_windows_unicode_and_root_paths() {
    let mut projected = snapshot();
    for (directory, expected) in [
        ("C:\\work\\herdr\\", "#1 herdr"),
        ("C:/work\\herdr/", "#1 herdr"),
        ("\\\\server\\share\\herdr\\", "#1 herdr"),
        ("/work/foo\\bar/", "#1 foo\\bar"),
        ("/work/foo\\", "#1 foo\\"),
        ("/Users/test/日本語", "#1 日本語"),
        ("/", "#1 /"),
    ] {
        projected.panes[0].foreground_cwd = Some(directory.into());
        assert_eq!(labels(&projected), [expected]);
    }
}

#[test]
fn unix_backslashes_do_not_merge_distinct_directories() {
    let mut projected = snapshot();
    projected.panes[0].cwd = Some("/work/foo".into());
    projected.panes[0].foreground_cwd = None;
    add_pane(&mut projected, "pane_backslash", Some("/work/foo\\"));
    add_pane(&mut projected, "pane_duplicate", Some("/work/foo\\/"));
    assert_eq!(labels(&projected), ["#1 foo / foo\\"]);
}

#[test]
fn rendered_tab_labels_update_in_background_and_click_the_same_tab_id() {
    let mut projected = snapshot();
    let mut second = projected.tabs[0].clone();
    second.tab_id = "tab_7".into();
    second.number = 7;
    second.focused = false;
    projected.tabs.push(second);
    add_pane(&mut projected, "pane_2", Some("/work/hoge"));
    projected.panes[1].tab_id = "tab_7".into();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected.clone()));
    state.set_pane_surface(surface());
    let frame = state.compose(140, 24).unwrap();
    let text = frame.cells[..140]
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect::<String>();
    assert!(text.contains("#1 repo"), "{text}");
    assert!(text.contains("#2 hoge"), "{text}");

    projected.revision += 1;
    projected.panes[1].cwd = Some("/work/dotfiles".into());
    state.set_snapshot(Box::new(projected));
    let mut updated_surface = surface();
    updated_surface.projection_revision = 2;
    state.set_pane_surface(updated_surface);
    let frame = state.compose(140, 24).unwrap();
    let text = frame.cells[..140]
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect::<String>();
    assert!(text.contains("#2 dotfiles"), "{text}");
    assert!(!text.contains("hoge"), "{text}");
    let rect = state
        .hits
        .tabs
        .iter()
        .find(|(_, id)| id == "tab_7")
        .unwrap()
        .0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 2,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let result = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: rect.x + 2,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(result.actions.iter().any(|action| matches!(action,
        ClientShellAction::Endpoint { request, .. }
        if matches!(&request.method, crate::api::schema::Method::TabFocus(target) if target.tab_id == "tab_7")
    )));
}

#[test]
fn tab_labels_stay_contiguous_after_closing_and_reordering_with_manual_names() {
    let mut projected = snapshot();
    projected.tabs[0].custom_label = true;
    projected.tabs[0].label = "review".into();
    for (number, directory) in [(7, "/work/hoge"), (20, "/work/other")] {
        let mut tab = projected.tabs[0].clone();
        tab.number = number;
        tab.tab_id = format!("tab_{number}");
        tab.custom_label = false;
        projected.tabs.push(tab);
        add_pane(&mut projected, &format!("pane_{number}"), Some(directory));
        projected.panes.last_mut().unwrap().tab_id = format!("tab_{number}");
    }
    assert_eq!(labels(&projected), ["review", "#2 hoge", "#3 other"]);
    projected.tabs.remove(1);
    assert_eq!(labels(&projected), ["review", "#2 other"]);
    projected.tabs.swap(0, 1);
    assert_eq!(labels(&projected), ["#1 other", "review"]);
    assert_eq!(projected.tabs[0].number, 20);
    assert_eq!(projected.tabs[0].tab_id, "tab_20");
    projected.tabs[1].custom_label = false;
    assert_eq!(labels(&projected), ["#1 other", "#2 repo"]);
}

#[test]
fn growing_directory_labels_keep_the_focused_tab_visible() {
    let mut projected = snapshot();
    for number in 2..=3 {
        let mut tab = projected.tabs[0].clone();
        tab.tab_id = format!("tab_{number}");
        tab.number = number;
        tab.focused = number == 3;
        projected.tabs.push(tab);
    }
    projected.tabs[0].focused = false;
    projected.focused_tab_id = Some("tab_3".into());
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected.clone()));
    state.set_pane_surface(surface());
    state.compose(80, 24).unwrap();
    assert_eq!(state.tab_scroll, 0);

    projected.revision += 1;
    projected.panes[0].foreground_cwd =
        Some("/work/a-directory-name-that-needs-a-lot-more-room".into());
    state.set_snapshot(Box::new(projected));
    let mut next_surface = surface();
    next_surface.projection_revision = 2;
    state.set_pane_surface(next_surface);
    state.compose(80, 24).unwrap();
    assert!(state.hits.tabs.iter().any(|(_, id)| id == "tab_3"));
    assert!(state.tab_scroll > 0);
}

#[test]
fn pane_focus_and_manual_tab_cwd_changes_do_not_reset_tab_scroll() {
    let mut previous = snapshot();
    let mut next = previous.clone();
    next.panes[0].focused = false;
    assert!(!super::super::directory_labels::tab_directory_labels_changed(&previous, &next));
    previous.tabs[0].custom_label = true;
    next.tabs[0].custom_label = true;
    next.panes[0].foreground_cwd = Some("/other/name".into());
    assert!(!super::super::directory_labels::tab_directory_labels_changed(&previous, &next));
}
