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
            matches!(&token.kind, crate::ui::ResolvedTokenKind::Workspace(value) if value == &format!("#1 {expected}"))
        ), "cwd {cwd:?}");
    }
}

#[test]
fn agent_primary_labels_prefix_the_tab_position_without_combining_panes() {
    let mut projected = directory_snapshot(4);
    projected.tabs.truncate(2);
    // Persistent numbers and raw API labels must not determine the displayed number.
    projected.tabs[1].number = 7;
    projected.tabs[1].label = "7".into();
    let expected = ["#1 herdr", "#1 anicru", "#2 anicru", "#2 herdr"];
    for (index, (tab_id, directory)) in [
        ("tab_1", "/work/herdr"),
        ("tab_1", "/work/anicru"),
        ("tab_2", "/work/anicru"),
        ("tab_2", "/work/herdr"),
    ]
    .iter()
    .enumerate()
    {
        projected.panes[index].tab_id = (*tab_id).into();
        projected.panes[index].foreground_cwd = Some((*directory).into());
        projected.agents[index].tab_id = (*tab_id).into();
    }
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected.clone()));
    state.set_pane_surface(surface());
    let frame = state.compose(120, 30).unwrap();
    let rows = frame_rows(&frame);
    assert_eq!(state.hits.agents.len(), 4);
    for ((rect, pane_id), label) in state.hits.agents.clone().iter().zip(expected) {
        let row = &rows[usize::from(rect.y)];
        assert!(row.contains(label), "{row}");
        assert!(!row.contains(" · "), "{row}");
        let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        })]);
        assert!(outcome.actions.iter().any(|action| matches!(action,
            ClientShellAction::Endpoint { request, .. }
            if matches!(&request.method, crate::api::schema::Method::PaneFocus(target) if &target.pane_id == pane_id)
        )));
    }

    // Even two agents in the same tab and directory remain two distinct entries.
    projected.revision += 1;
    projected.panes[1].foreground_cwd = Some("/work/herdr".into());
    state.set_snapshot(Box::new(projected));
    let mut pane_surface = surface();
    pane_surface.projection_revision = 2;
    state.set_pane_surface(pane_surface);
    let frame = state.compose(120, 30).unwrap();
    let rows = frame_rows(&frame);
    assert_eq!(state.hits.agents.len(), 4);
    for (rect, _) in &state.hits.agents[..2] {
        assert!(rows[usize::from(rect.y)].contains("#1 herdr"));
    }
}

#[test]
fn single_tab_agents_show_the_number_and_keep_manual_tab_names() {
    let mut projected = directory_snapshot(1);
    let config = ClientShellConfig::from_config(&Config::default());
    for custom in [false, true] {
        projected.tabs[0].custom_label = custom;
        projected.tabs[0].label = "review".into();
        let row = agent_sidebar::agent_row(&projected, "pane_1", &config, None).unwrap();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 2));
        agent_sidebar::render_agent_row(&mut buffer, Rect::new(0, 0, 50, 2), &row, &config);
        let line = (0..50).map(|x| buffer[(x, 0)].symbol()).collect::<String>();
        assert!(line.contains("#1 project-1"), "{line}");
        assert_eq!(line.contains(" · review"), custom, "{line}");
    }
}

#[test]
fn agent_number_prefix_keeps_directory_style_rules_working() {
    let config: Config = toml::from_str(
        r#"
[ui.sidebar.agents]
rows = [[{ token = "workspace", rules = [{ equals = "project-1", bold = true }] }]]
"#,
    )
    .unwrap();
    let row = agent_sidebar::agent_row(
        &directory_snapshot(1),
        "pane_1",
        &ClientShellConfig::from_config(&config),
        None,
    )
    .unwrap();
    assert_eq!(row.rows[0][0].style.bold, Some(true));
    assert!(
        matches!(&row.rows[0][0].kind, crate::ui::ResolvedTokenKind::Workspace(label) if label == "#1 project-1")
    );
}

#[test]
fn tab_sidebar_keeps_agents_during_resize_and_routes_remote_clicks() {
    use crate::client::endpoint::{ClientEndpointStatus, ProfileId, SavedSshEndpoint};
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    // Leave room for the machine label and the new tab-number prefix.
    state.sidebar_width = 36;
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
    assert!(text.contains(" machines"));
    assert!(state.hits.workspaces.is_empty());
}

#[test]
fn narrow_agents_follow_each_pane_directory_and_keep_click_targets() {
    let mut projected = directory_snapshot(2);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    for directory in ["/work/foo\\bar", "/work/changed"] {
        projected.revision += 1;
        projected.panes[1].foreground_cwd = Some(directory.into());
        state.set_snapshot(Box::new(projected.clone()));
        let mut pane_surface = surface();
        pane_surface.projection_revision = projected.revision;
        state.set_pane_surface(pane_surface);
        state.mode = ClientShellMode::Navigate;
        let frame = state.compose(44, 30).unwrap();
        let (rect, _) = state.hits.mobile_targets.iter().find(|(_, target)| {
            matches!(target, ClientMobileTarget::Agent { pane_id, .. } if pane_id == "pane_2")
        }).unwrap();
        let rect = *rect;
        let rows = frame_rows(&frame);
        assert!(rows[usize::from(rect.y)]
            .contains(super::super::directory_labels::directory_name(directory)));
        assert!(rows[usize::from(rect.y) + 1].contains("blocked"));
        assert!(rows[usize::from(rect.y) + 1].contains("codex"));
        let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x + 1,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        })]);
        assert!(outcome.actions.iter().any(|action| matches!(action,
            ClientShellAction::Endpoint { request, .. }
            if matches!(&request.method, crate::api::schema::Method::PaneFocus(target) if target.pane_id == "pane_2")
        )));
    }
}

#[test]
fn agentless_machines_remain_clickable_without_spaces_or_a_local_surface() {
    use crate::client::endpoint::{ClientEndpointStatus, ProfileId, SavedSshEndpoint};
    for collapsed in [false, true] {
        for available in [false, true] {
            let mut state =
                ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
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
            let mut remote = snapshot();
            remote.boot_id = "remote-boot".into();
            state.set_endpoint_snapshot(&remote_id, Box::new(remote));
            if available {
                state.set_snapshot(Box::new(snapshot()));
                state.set_pane_surface(surface());
            }
            state.sidebar_collapsed = collapsed;
            let frame = state.compose(106, 30).unwrap();
            assert!(state.hits.workspaces.is_empty());
            assert!(!rendered_text(&frame).contains(" spaces"));
            let rect = state
                .hits
                .machines
                .iter()
                .find(|hit| hit.endpoint_id == remote_id)
                .unwrap()
                .rect;
            let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: rect.x,
                row: rect.y,
                modifiers: KeyModifiers::empty(),
            })]);
            assert!(
                matches!(&outcome.actions[..],
                    [ClientShellAction::ActivateEndpoint { endpoint_id, .. }] if endpoint_id == &remote_id
                ),
                "collapsed={collapsed}, available={available}: {:?}",
                outcome.actions
            );
            state.set_endpoint_status(&remote_id, ClientEndpointStatus::Reconnecting);
            state.compose(106, 30).unwrap();
            let rect = state
                .hits
                .machines
                .iter()
                .find(|hit| hit.endpoint_id == remote_id)
                .unwrap()
                .rect;
            state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: rect.x,
                row: rect.y,
                modifiers: KeyModifiers::empty(),
            })]);
            assert_eq!(state.take_machine_retries(), [remote_id]);
        }
    }
}

#[test]
fn machine_selector_scroll_reaches_endpoints_beyond_the_available_rows() {
    use crate::client::endpoint::{ClientEndpointStatus, ProfileId, SavedSshEndpoint};
    let profiles = (1..=15)
        .map(|index| SavedSshEndpoint {
            id: ProfileId::parse(format!("{index:032x}")).unwrap(),
            label: format!("Build {index}"),
            target: "test@example.invalid".into(),
            session: "test".into(),
            enabled: true,
        })
        .collect::<Vec<_>>();
    let last_id = ClientEndpointId::Ssh(profiles.last().unwrap().id.clone());
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_endpoint_catalog(&profiles);
    for profile in &profiles {
        state.set_endpoint_status(
            &ClientEndpointId::Ssh(profile.id.clone()),
            ClientEndpointStatus::Online,
        );
        state.set_endpoint_snapshot(
            &ClientEndpointId::Ssh(profile.id.clone()),
            Box::new(snapshot()),
        );
    }
    state.set_snapshot(Box::new(directory_snapshot(2)));
    state.set_pane_surface(surface());
    state.compose(106, 24).unwrap();
    assert_eq!(state.hits.agent_sort_toggle.y, 0);
    assert!(!state.hits.workspace_scrollbar.is_empty());
    let body = state.hits.workspace_body;
    for _ in 0..20 {
        state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: body.x,
            row: body.y,
            modifiers: KeyModifiers::empty(),
        })]);
        state.compose(106, 24).unwrap();
    }
    let rect = state
        .hits
        .machines
        .iter()
        .find(|hit| hit.endpoint_id == last_id)
        .unwrap()
        .rect;
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(
        matches!(&outcome.actions[..], [ClientShellAction::ActivateEndpoint { endpoint_id, .. }] if endpoint_id == &last_id),
        "{:?}",
        outcome.actions
    );
}

#[test]
fn narrow_remote_agents_use_the_matching_endpoint_directory_and_workspace_fallback() {
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
    state.set_snapshot(Box::new(directory_snapshot(1)));
    state.set_pane_surface(surface());
    let mut remote = directory_snapshot(1);
    remote.boot_id = "remote-boot".into();
    remote.panes[0].cwd = Some("C:\\work\\remote-project".into());
    remote.panes[0].foreground_cwd = None;
    for expected in ["remote-project", "client-shell"] {
        state.set_endpoint_snapshot(&remote_id, Box::new(remote.clone()));
        state.mode = ClientShellMode::Navigate;
        let frame = state.compose(44, 40).unwrap();
        let (rect, _) = state.hits.mobile_targets.iter().find(|(_, target)| {
            matches!(target, ClientMobileTarget::Agent { endpoint_id, .. } if endpoint_id == &remote_id)
        }).unwrap();
        let row = &frame_rows(&frame)[usize::from(rect.y)];
        assert!(row.contains(expected), "{row}");
        remote.panes[0].cwd = None;
    }
}
