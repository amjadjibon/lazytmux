use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use std::path::PathBuf;

pub const FIELD_DELIMITER: char = '\t';

pub fn parse_sessions(output: &str) -> Vec<Session> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(FIELD_DELIMITER).collect();
            if fields.len() < 4 {
                return None;
            }
            let id = SessionId::from(fields[0]);
            let name = fields[1].to_string();
            let window_count = fields[2].parse::<usize>().unwrap_or(0);
            let attached = fields[3] == "1";

            Some(Session::new(id, name, window_count, attached))
        })
        .collect()
}

pub fn parse_windows(output: &str) -> Vec<(SessionId, Window)> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(FIELD_DELIMITER).collect();
            if fields.len() < 7 {
                return None;
            }
            let session_id = SessionId::from(fields[0]);
            let window_id = WindowId::from(fields[1]);
            let index = fields[2].parse::<u32>().unwrap_or(0);
            let name = fields[3].to_string();
            let active = fields[4] == "1";
            let layout_str = fields[6].to_string();

            let window = Window::new(window_id, session_id.clone(), index, name, active, layout_str);
            Some((session_id, window))
        })
        .collect()
}

pub fn parse_panes(output: &str) -> Vec<(SessionId, WindowId, Pane)> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(FIELD_DELIMITER).collect();
            if fields.len() < 9 {
                return None;
            }
            let session_id = SessionId::from(fields[0]);
            let window_id = WindowId::from(fields[1]);
            let pane_id = PaneId::from(fields[2]);
            let index = fields[3].parse::<u32>().unwrap_or(0);
            let active = fields[4] == "1";
            let command = fields[5].to_string();
            let path = PathBuf::from(fields[6]);
            let width = fields[7].parse::<u16>().unwrap_or(80);
            let height = fields[8].parse::<u16>().unwrap_or(24);

            let pane = Pane::new(
                pane_id,
                window_id.clone(),
                session_id.clone(),
                index,
                active,
                command,
                path,
                width,
                height,
            );
            Some((session_id, window_id, pane))
        })
        .collect()
}

pub fn assemble_tree(
    mut sessions: Vec<Session>,
    windows: Vec<(SessionId, Window)>,
    panes: Vec<(SessionId, WindowId, Pane)>,
) -> Vec<Session> {
    // Group windows into a map or assign them to sessions
    let mut windows_by_session: std::collections::HashMap<SessionId, Vec<Window>> =
        std::collections::HashMap::new();

    for (s_id, w) in windows {
        windows_by_session.entry(s_id).or_default().push(w);
    }

    // Group panes by window
    let mut panes_by_window: std::collections::HashMap<WindowId, Vec<Pane>> =
        std::collections::HashMap::new();

    for (_s_id, w_id, p) in panes {
        panes_by_window.entry(w_id).or_default().push(p);
    }

    // Attach panes to windows
    for windows_list in windows_by_session.values_mut() {
        for window in windows_list.iter_mut() {
            if let Some(p_list) = panes_by_window.remove(&window.id) {
                window.panes = p_list;
                window.panes.sort_by_key(|p| p.index);
            }
        }
        windows_list.sort_by_key(|w| w.index);
    }

    // Attach windows to sessions
    for session in sessions.iter_mut() {
        if let Some(w_list) = windows_by_session.remove(&session.id) {
            session.window_count = w_list.len();
            session.windows = w_list;
        }
    }

    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sessions() {
        let raw = "$0\twork\t4\t1\n$1\tpersonal\t2\t0\n";
        let sessions = parse_sessions(raw);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "work");
        assert!(sessions[0].attached);
        assert_eq!(sessions[1].name, "personal");
        assert!(!sessions[1].attached);
    }

    #[test]
    fn test_assemble_tree_full_hierarchy() {
        let sess_raw = "$0\tprod\t2\t1\n$1\tdev\t1\t0\n";
        let win_raw = "$0\t@1\t1\tweb\t1\t2\t100x50,0,0\n$0\t@2\t2\tdb\t0\t1\t100x50,0,0\n$1\t@3\t1\teditor\t1\t1\t100x50,0,0\n";
        let pane_raw = "$0\t@1\t%1\t0\t1\tnode\t/app\t50\t50\n$0\t@1\t%2\t1\t0\tredis\t/app\t50\t50\n$0\t@2\t%3\t0\t1\tpsql\t/db\t100\t50\n$1\t@3\t%4\t0\t1\tnvim\t/dev\t100\t50\n";

        let sessions = parse_sessions(sess_raw);
        let windows = parse_windows(win_raw);
        let panes = parse_panes(pane_raw);

        let tree = assemble_tree(sessions, windows, panes);
        assert_eq!(tree.len(), 2);

        // Prod session
        assert_eq!(tree[0].name, "prod");
        assert_eq!(tree[0].windows.len(), 2);
        assert_eq!(tree[0].windows[0].name, "web");
        assert_eq!(tree[0].windows[0].panes.len(), 2);
        assert_eq!(tree[0].windows[0].panes[0].current_command, "node");
        assert_eq!(tree[0].windows[1].name, "db");
        assert_eq!(tree[0].windows[1].panes.len(), 1);

        // Dev session
        assert_eq!(tree[1].name, "dev");
        assert_eq!(tree[1].windows.len(), 1);
        assert_eq!(tree[1].windows[0].panes.len(), 1);
    }

    #[test]
    fn test_parse_empty_input() {
        assert!(parse_sessions("").is_empty());
        assert!(parse_sessions("   \n\n  ").is_empty());
        assert!(parse_windows("").is_empty());
        assert!(parse_panes("").is_empty());
    }

    #[test]
    fn test_parse_windows_and_panes_with_special_chars() {
        let win_raw = "$0\t@1\t1\tmy window | with spaces\t1\t2\t100x50,0,0\n";
        let windows = parse_windows(win_raw);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].1.name, "my window | with spaces");

        let pane_raw = "$0\t@1\t%1\t0\t1\tnvim /path/with spaces/file.rs\t/Users/test/folder with | pipe\t120\t40\n";
        let panes = parse_panes(pane_raw);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].2.current_command, "nvim /path/with spaces/file.rs");
        assert_eq!(panes[0].2.current_path, PathBuf::from("/Users/test/folder with | pipe"));
    }

    #[test]
    fn test_unicode_and_emojis() {
        let sess_raw = "$0\t🚀 prod-app (日本語)\t1\t1\n";
        let sessions = parse_sessions(sess_raw);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "🚀 prod-app (日本語)");
    }
}
