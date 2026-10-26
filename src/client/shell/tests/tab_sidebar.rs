use super::*;

fn directory_snapshot(count: usize) -> ClientShellSnapshot {
    let mut projected = snapshot();
    for index in 1..=count {
        if index > 1 {
            let mut tab = projected.tabs[0].clone();
            tab.tab_id = format!("tab_{index}");
            tab.label = index.to_string();
            tab.focused = false;
            projected.tabs.push(tab);
            let mut pane = projected.panes[0].clone();
            pane.pane_id = format!("pane_{index}");
            pane.tab_id = format!("tab_{index}");
            pane.focused = false;
            projected.panes.push(pane);
        }
        let pane = &mut projected.panes[index - 1];
        pane.cwd = Some(format!("/Users/test/Develop/project-{index}"));
        pane.foreground_cwd = pane.cwd.clone();
        projected.agents.push(ClientShellAgent {
            pane_id: pane.pane_id.clone(),
            workspace_id: pane.workspace_id.clone(),
            tab_id: pane.tab_id.clone(),
            name: None,
            display_agent: None,
            agent: Some("codex".into()),
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: if index == 1 {
                AgentStatus::Working
            } else {
                AgentStatus::Blocked
            },
            state_change_seq: index as u64,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: index == 1,
        });
    }
    projected
}

fn rendered_text(frame: &FrameData) -> String {
    frame
        .cells
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect()
}

#[test]
fn tab_sidebar_uses_pane_directories_and_follows_cd_in_background_tabs() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut projected = directory_snapshot(2);
    // Foreground process cwd is preferred over an older shell cwd.
    projected.panes[1].foreground_cwd = Some("/Users/test/Develop/dotfiles/".into());
    state.set_snapshot(Box::new(projected.clone()));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 30).unwrap();
    let text = rendered_text(&frame);
    for expected in [
        "agents",
        "project-1",
        "dotfiles",
        "codex",
        "working",
        "blocked",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(!text.contains(" spaces"));
    assert!(!text.contains("client-shell"));
    assert!(state.hits.workspaces.is_empty());
    assert!(state.hits.new_workspace.is_empty());
    assert!(state.hits.sidebar_section_divider.is_empty());
    assert_eq!(state.hits.agent_sort_toggle.y, 0);
    assert_eq!(
        state.hits.agent_body.bottom(),
        state.layout(106, 30).sidebar.bottom() - 1
    );
    assert!(!state.hits.global_launcher.is_empty());
    assert!(!state.hits.sidebar_toggle.is_empty());

    projected.revision += 1;
    projected.panes[1].foreground_cwd = None;
    projected.panes[1].cwd = Some("/Users/test/Develop/renamed".into());
    state.set_snapshot(Box::new(projected));
    let mut updated_surface = surface();
    updated_surface.projection_revision = 2;
    state.set_pane_surface(updated_surface);
    let text = rendered_text(&state.compose(106, 30).unwrap());
    assert!(text.contains("renamed"));
    assert!(!text.contains("dotfiles"));
}

#[test]
fn tab_sidebar_clicks_focus_the_agent_pane_and_tabs_keep_their_ids() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(directory_snapshot(2)));
    state.set_pane_surface(surface());
    state.compose(106, 30).unwrap();
    let rect = state
        .hits
        .agents
        .iter()
        .find(|(_, id)| id == "pane_2")
        .unwrap()
        .0;
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(
        matches!(&outcome.actions[..], [ClientShellAction::Endpoint { request, .. }]
        if matches!(&request.method, crate::api::schema::Method::PaneFocus(target) if target.pane_id == "pane_2"))
    );
    assert!(state.hits.tabs.iter().any(|(_, id)| id == "tab_2"));

    state.sidebar_collapsed = true;
    state.compose(106, 30).unwrap();
    assert!(state.hits.workspaces.is_empty());
    assert_eq!(state.hits.agents[0].0.y, 0);
    assert_eq!(state.hits.agents[1].1, "pane_2");
}

#[test]
fn tab_sidebar_scroll_reaches_background_tab_agents() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(directory_snapshot(15)));
    state.set_pane_surface(surface());
    state.compose(106, 16).unwrap();
    let body = state.hits.agent_body;
    assert!(!state.hits.agent_scrollbar.is_empty());
    for _ in 0..20 {
        state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: body.x,
            row: body.y,
            modifiers: KeyModifiers::empty(),
        })]);
        state.compose(106, 16).unwrap();
    }
    assert!(state.hits.agents.iter().any(|(_, id)| id == "pane_15"));
    assert!(state.agent_scroll > 0);
}

#[test]
fn tab_sidebar_directory_labels_handle_remote_paths_roots_and_missing_cwd() {
    let config = ClientShellConfig::from_config(&Config::default());
    let mut projected = directory_snapshot(1);
    for (cwd, expected) in [
        (Some("C:\\Users\\test\\dotfiles\\"), "dotfiles"),
        (Some("/"), "/"),
        (Some("/Users/test/日本語"), "日本語"),
        (Some(""), "client-shell"),
        (None, "client-shell"),
    ] {
        projected.panes[0].foreground_cwd = None;
        projected.panes[0].cwd = cwd.map(str::to_string);
        let row = agent_sidebar::agent_row(&projected, "pane_1", &config, None).unwrap();
        assert!(row.rows.iter().flatten().any(|token|
            matches!(&token.kind, crate::ui::ResolvedTokenKind::Workspace(value) if value == expected)
        ), "cwd {cwd:?}");
    }
}

#[test]
fn tab_sidebar_keeps_agents_during_resize_and_routes_remote_clicks() {
    use crate::client::endpoint::{ClientEndpointStatus, ProfileId, SavedSshEndpoint};
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let profile = SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Build".into(),
        target: "test@example.invalid".into(),
        session: "test".into(),
        enabled: true,
    };
    let remote_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(&remote_id, ClientEndpointStatus::Online);
    let mut remote = directory_snapshot(1);
    remote.boot_id = "remote-boot".into();
    remote.panes[0].foreground_cwd = Some("C:\\work\\remote-project".into());
    state.set_endpoint_snapshot(&remote_id, Box::new(remote));
    state.set_snapshot(Box::new(directory_snapshot(1)));
    state.set_pane_surface(surface());
    let text = rendered_text(&state.compose(106, 30).unwrap());
    assert!(text.contains("remote-project"));
    assert!(state.hits.workspaces.is_empty());
    let (rect, _, _) = state
        .hits
        .endpoint_agents
        .iter()
        .find(|(_, endpoint, _)| endpoint == &remote_id)
        .unwrap()
        .clone();
    let result = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(&result.actions[..],
        [ClientShellAction::ActivateEndpoint {
            endpoint_id,
            target: Some(ClientEndpointFocusTarget::Pane(pane_id)),
        }] if endpoint_id == &remote_id && pane_id == "pane_1"
    ));

    // A missing surface uses the connection/resize composition path.
    state.pane_surface = None;
    let text = rendered_text(&state.compose(106, 30).unwrap());
    assert!(text.contains("agents"));
    assert!(!text.contains(" spaces"));
    assert!(!text.contains(" machines"));
    assert!(state.hits.workspaces.is_empty());
}
