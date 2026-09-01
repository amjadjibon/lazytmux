use super::client::TmuxClient;
use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MockTmuxClient {
    pub sessions: Vec<Session>,
    pub counter: usize,
}

impl Default for MockTmuxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTmuxClient {
    pub fn new() -> Self {
        let mut client = Self {
            sessions: Vec::new(),
            counter: 10,
        };
        client.populate_mock_data();
        client
    }

    fn populate_mock_data(&mut self) {
        // Session 1: "work"
        let s1_id = SessionId::from("$1");
        let mut s1 = Session::new(s1_id.clone(), "work".to_string(), 4, true);
        s1.is_favorite = true;

        // Window 1.1: "editor"
        let w1_id = WindowId::from("@1");
        let mut w1 = Window::new(
            w1_id.clone(),
            s1_id.clone(),
            1,
            "editor".to_string(),
            true,
            "100x50,0,0".to_string(),
        );
        let mut p1 = Pane::new(
            PaneId::from("%1"),
            w1_id.clone(),
            s1_id.clone(),
            1,
            true,
            "nvim".to_string(),
            PathBuf::from("~/code/lazytmux"),
            100,
            50,
        );
        let nvim_ansi = b"\x1b[38;5;39m1\x1b[0m \x1b[38;5;208mpub struct\x1b[0m \x1b[38;5;220mLazyTmux\x1b[0m {\n\x1b[38;5;39m2\x1b[0m     \x1b[38;5;246m// High-performance visual workspace explorer\x1b[0m\n\x1b[38;5;39m3\x1b[0m     \x1b[38;5;208mpub\x1b[0m session: \x1b[38;5;220mSession\x1b[0m,\n\x1b[38;5;39m4\x1b[0m     \x1b[38;5;208mpub\x1b[0m windows: \x1b[38;5;220mVec\x1b[0m<\x1b[38;5;220mWindow\x1b[0m>,\n\x1b[38;5;39m5\x1b[0m }\n\x1b[38;5;39m6\x1b[0m \n\x1b[38;5;39m7\x1b[0m \x1b[38;5;208mimpl\x1b[0m \x1b[38;5;220mLazyTmux\x1b[0m {\n\x1b[38;5;39m8\x1b[0m     \x1b[38;5;208mpub fn\x1b[0m \x1b[38;5;120mrun\x1b[0m(&\x1b[38;5;208mself\x1b[0m) -> \x1b[38;5;220mResult\x1b[0m<()> {\n\x1b[38;5;39m9\x1b[0m         \x1b[38;5;220mOk\x1b[0m(())\n\x1b[38;5;39m10\x1b[0m    }\n\x1b[38;5;39m11\x1b[0m }";
        p1.set_preview(nvim_ansi.to_vec());

        let mut p2 = Pane::new(
            PaneId::from("%2"),
            w1_id.clone(),
            s1_id.clone(),
            2,
            false,
            "cargo".to_string(),
            PathBuf::from("~/code/lazytmux"),
            100,
            50,
        );
        let cargo_ansi = b"\x1b[38;5;245m$ cargo test\x1b[0m\n   \x1b[32;1mCompiling\x1b[0m lazytmux v0.1.0\n    \x1b[32;1mFinished\x1b[0m test [unoptimized + debuginfo] in 0.84s\n     \x1b[32;1mRunning\x1b[0m unittests src/main.rs\n\nrunning 5 tests\ntest tmux::parser::test_parse_sessions ... \x1b[32mok\x1b[0m\ntest tmux::parser::test_parse_windows ... \x1b[32mok\x1b[0m\ntest ui::layout::test_split ... \x1b[32mok\x1b[0m\n\ntest result: \x1b[32;1mok\x1b[0m. 3 passed; 0 failed; 0 ignored";
        p2.set_preview(cargo_ansi.to_vec());
        w1.panes = vec![p1, p2];

        // Window 1.2: "backend"
        let w2_id = WindowId::from("@2");
        let mut w2 = Window::new(
            w2_id.clone(),
            s1_id.clone(),
            2,
            "backend".to_string(),
            false,
            "100x50,0,0".to_string(),
        );
        let mut p3 = Pane::new(
            PaneId::from("%3"),
            w2_id.clone(),
            s1_id.clone(),
            1,
            true,
            "./server".to_string(),
            PathBuf::from("~/code/api"),
            100,
            50,
        );
        let server_ansi = b"\x1b[34m[INFO]\x1b[0m 2026-09-01T16:50:00Z Actix-web server listening on \x1b[32m127.0.0.1:8080\x1b[0m\n\x1b[34m[INFO]\x1b[0m 2026-09-01T16:50:02Z GET /api/v1/health -> \x1b[32m200 OK\x1b[0m (1.2ms)\n\x1b[34m[INFO]\x1b[0m 2026-09-01T16:50:05Z GET /api/v1/sessions -> \x1b[32m200 OK\x1b[0m (4.8ms)";
        p3.set_preview(server_ansi.to_vec());
        w2.panes = vec![p3];

        // Window 1.3: "logs"
        let w3_id = WindowId::from("@3");
        let mut w3 = Window::new(
            w3_id.clone(),
            s1_id.clone(),
            3,
            "logs".to_string(),
            false,
            "100x50,0,0".to_string(),
        );
        let mut p4 = Pane::new(
            PaneId::from("%4"),
            w3_id.clone(),
            s1_id.clone(),
            1,
            true,
            "docker compose logs -f".to_string(),
            PathBuf::from("~/code/infra"),
            100,
            50,
        );
        let docker_ansi = b"\x1b[36mpostgres-1  |\x1b[0m database system is ready to accept connections\n\x1b[35mredis-1     |\x1b[0m Ready to accept connections tcp 6379\n\x1b[33mclickhouse  |\x1b[0m Ready for connections at 0.0.0.0:9000";
        p4.set_preview(docker_ansi.to_vec());
        w3.panes = vec![p4];

        // Window 1.4: "shell"
        let w4_id = WindowId::from("@4");
        let mut w4 = Window::new(
            w4_id.clone(),
            s1_id.clone(),
            4,
            "shell".to_string(),
            false,
            "100x50,0,0".to_string(),
        );
        let mut p5 = Pane::new(
            PaneId::from("%5"),
            w4_id.clone(),
            s1_id.clone(),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("~/code/lazytmux"),
            100,
            50,
        );
        let zsh_ansi = b"\x1b[32mamjad@macbook\x1b[0m:\x1b[34m~/code/lazytmux\x1b[0m (main)\n$ git status\nOn branch main\nChanges not staged for commit:\n  modified:   src/main.rs\n\nno changes added to commit";
        p5.set_preview(zsh_ansi.to_vec());
        w4.panes = vec![p5];

        s1.windows = vec![w1, w2, w3, w4];

        // Session 2: "personal"
        let s2_id = SessionId::from("$2");
        let mut s2 = Session::new(s2_id.clone(), "personal".to_string(), 2, false);
        let w5_id = WindowId::from("@5");
        let mut w5 = Window::new(
            w5_id.clone(),
            s2_id.clone(),
            1,
            "blog".to_string(),
            true,
            "100x50,0,0".to_string(),
        );
        let mut p6 = Pane::new(
            PaneId::from("%6"),
            w5_id.clone(),
            s2_id.clone(),
            1,
            true,
            "hugo server".to_string(),
            PathBuf::from("~/personal/blog"),
            100,
            50,
        );
        let hugo_ansi = b"Watching for config changes in ~/personal/blog/hugo.toml\n\x1b[32mWeb Server is available at http://localhost:1313/\x1b[0m (bind address 127.0.0.1)\nPress Ctrl+C to stop";
        p6.set_preview(hugo_ansi.to_vec());
        w5.panes = vec![p6];
        s2.windows = vec![w5];

        // Session 3: "infra"
        let s3_id = SessionId::from("$3");
        let mut s3 = Session::new(s3_id.clone(), "infra".to_string(), 2, false);
        let w6_id = WindowId::from("@6");
        let mut w6 = Window::new(
            w6_id.clone(),
            s3_id.clone(),
            1,
            "terraform".to_string(),
            true,
            "100x50,0,0".to_string(),
        );
        let mut p7 = Pane::new(
            PaneId::from("%7"),
            w6_id.clone(),
            s3_id.clone(),
            1,
            true,
            "terraform plan".to_string(),
            PathBuf::from("~/infra/aws"),
            100,
            50,
        );
        let tf_ansi = b"\x1b[32mPlan:\x1b[0m 2 to add, 1 to change, 0 to destroy.\n\n\x1b[33m-----------------------------------------------------------------------------\x1b[0m\n\nNote: You didn't use the -out option to save this plan.";
        p7.set_preview(tf_ansi.to_vec());
        w6.panes = vec![p7];
        s3.windows = vec![w6];

        self.sessions = vec![s1, s2, s3];
    }
}

impl TmuxClient for MockTmuxClient {
    fn list_sessions(&self) -> Result<Vec<Session>> {
        Ok(self.sessions.clone())
    }

    fn list_windows(&self, session: &SessionId) -> Result<Vec<Window>> {
        if let Some(s) = self.sessions.iter().find(|s| &s.id == session) {
            Ok(s.windows.clone())
        } else {
            Err(anyhow!("Session not found"))
        }
    }

    fn list_panes(&self, window: &WindowId) -> Result<Vec<Pane>> {
        for s in &self.sessions {
            if let Some(w) = s.windows.iter().find(|w| &w.id == window) {
                return Ok(w.panes.clone());
            }
        }
        Err(anyhow!("Window not found"))
    }

    fn fetch_full_tree(&self) -> Result<Vec<Session>> {
        Ok(self.sessions.clone())
    }

    fn capture_pane(&self, pane: &PaneId, _lines: usize, _preserve_ansi: bool) -> Result<Vec<u8>> {
        for s in &self.sessions {
            for w in &s.windows {
                if let Some(p) = w.panes.iter().find(|p| &p.id == pane) {
                    return Ok(p.preview_raw.clone());
                }
            }
        }
        Ok(b"No pane output found".to_vec())
    }

    fn create_session(&mut self, name: &str) -> Result<SessionId> {
        self.counter += 1;
        let id = SessionId(format!("${}", self.counter));
        let w_id = WindowId(format!("@{}", self.counter * 10));
        let p_id = PaneId(format!("%{}", self.counter * 100));

        let pane = Pane::new(
            p_id,
            w_id.clone(),
            id.clone(),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("~"),
            80,
            24,
        );
        let mut win = Window::new(
            w_id,
            id.clone(),
            1,
            "main".to_string(),
            true,
            "80x24,0,0".to_string(),
        );
        win.panes = vec![pane];

        let mut sess = Session::new(id.clone(), name.to_string(), 1, false);
        sess.windows = vec![win];

        self.sessions.push(sess);
        Ok(id)
    }

    fn rename_session(&mut self, session: &SessionId, new_name: &str) -> Result<()> {
        if let Some(s) = self.sessions.iter_mut().find(|s| &s.id == session) {
            s.name = new_name.to_string();
            Ok(())
        } else {
            Err(anyhow!("Session not found"))
        }
    }

    fn kill_session(&mut self, session: &SessionId) -> Result<()> {
        self.sessions.retain(|s| &s.id != session);
        Ok(())
    }

    fn create_window(&mut self, session: &SessionId, name: &str) -> Result<WindowId> {
        self.counter += 1;
        let w_id = WindowId(format!("@{}", self.counter * 10));
        let p_id = PaneId(format!("%{}", self.counter * 100));

        if let Some(s) = self.sessions.iter_mut().find(|s| &s.id == session) {
            let next_idx = s.windows.len() as u32 + 1;
            let pane = Pane::new(
                p_id,
                w_id.clone(),
                session.clone(),
                1,
                true,
                "zsh".to_string(),
                PathBuf::from("~"),
                80,
                24,
            );
            let mut win = Window::new(
                w_id.clone(),
                session.clone(),
                next_idx,
                name.to_string(),
                true,
                "80x24,0,0".to_string(),
            );
            win.panes = vec![pane];
            s.windows.push(win);
            s.window_count = s.windows.len();
            Ok(w_id)
        } else {
            Err(anyhow!("Session not found"))
        }
    }

    fn rename_window(&mut self, window: &WindowId, new_name: &str) -> Result<()> {
        for s in &mut self.sessions {
            if let Some(w) = s.windows.iter_mut().find(|w| &w.id == window) {
                w.name = new_name.to_string();
                return Ok(());
            }
        }
        Err(anyhow!("Window not found"))
    }

    fn kill_window(&mut self, window: &WindowId) -> Result<()> {
        for s in &mut self.sessions {
            s.windows.retain(|w| &w.id != window);
            s.window_count = s.windows.len();
        }
        Ok(())
    }

    fn kill_pane(&mut self, pane: &PaneId) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                w.panes.retain(|p| &p.id != pane);
            }
        }
        Ok(())
    }

    fn zoom_pane(&mut self, _pane: &PaneId) -> Result<()> {
        Ok(())
    }

    fn focus_pane(&self, _session: &SessionId, _window: &WindowId, _pane: &PaneId) -> Result<()> {
        Ok(())
    }
}
