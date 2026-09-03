use super::client::{ResizeDirection, TmuxClient};
use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MockTmuxClient {
    pub sessions: Vec<Session>,
    pub counter: usize,
    pub synced_windows: HashSet<WindowId>,
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
            counter: 20,
            synced_windows: HashSet::new(),
        };
        client.populate_mock_data();
        client
    }

    fn populate_mock_data(&mut self) {
        // -------------------------------------------------------------
        // Session 1: "work" (Active Workspace, Favorited)
        // -------------------------------------------------------------
        let s1_id = SessionId::from("$1");
        let mut s1 = Session::new(s1_id.clone(), "work".to_string(), 4, true);
        s1.is_favorite = true;

        // Window 1.1: "editor" (Active window, 2 panes: nvim + cargo test)
        let w1_id = WindowId::from("@1");
        let mut w1 = Window::new(
            w1_id.clone(),
            s1_id.clone(),
            1,
            "editor".to_string(),
            true,
            "".to_string(),
        );

        let mut p1 = Pane::new(
            PaneId::from("%1"),
            w1_id.clone(),
            s1_id.clone(),
            1,
            true,
            "nvim".to_string(),
            PathBuf::from("~/github.com/amjadjibon/lazytmux"),
            80,
            50,
        );
        p1.git_branch = Some("main".to_string());
        p1.set_preview(
            b"\x1b[38;5;240m   1\x1b[0m \x1b[38;5;208muse\x1b[0m color_eyre::Result;\n\
\x1b[38;5;240m   2\x1b[0m \x1b[38;5;208muse\x1b[0m lazytmux::app::App;\n\
\x1b[38;5;240m   3\x1b[0m \x1b[38;5;208muse\x1b[0m lazytmux::config::Config;\n\
\x1b[38;5;240m   4\x1b[0m \x1b[38;5;208muse\x1b[0m lazytmux::tmux::{CliTmuxClient, MockTmuxClient};\n\
\x1b[38;5;240m   5\x1b[0m \n\
\x1b[38;5;240m   6\x1b[0m \x1b[38;5;246m/// Entry point for LazyTmux visual workspace explorer\x1b[0m\n\
\x1b[38;5;240m   7\x1b[0m \x1b[38;5;208mpub fn\x1b[0m \x1b[38;5;120mmain\x1b[0m() -> \x1b[38;5;220mResult\x1b[0m<()> {\n\
\x1b[38;5;240m   8\x1b[0m     color_eyre::install()?;\n\
\x1b[38;5;240m   9\x1b[0m     \x1b[38;5;208mlet\x1b[0m config = Config::load_or_default();\n\
\x1b[38;5;240m  10\x1b[0m     \x1b[38;5;208mlet\x1b[0m is_mock = std::env::args().any(|a| a == \x1b[38;5;142m\"--mock\"\x1b[0m);\n\
\x1b[38;5;240m  11\x1b[0m \n\
\x1b[38;5;240m  12\x1b[0m     \x1b[38;5;208mlet\x1b[0m client: \x1b[38;5;220mBox\x1b[0m<\x1b[38;5;208mdyn\x1b[0m \x1b[38;5;220mTmuxClient\x1b[0m> = \x1b[38;5;208mif\x1b[0m is_mock {\n\
\x1b[38;5;240m  13\x1b[0m         \x1b[38;5;220mBox\x1b[0m::new(MockTmuxClient::new())\n\
\x1b[38;5;240m  14\x1b[0m     } \x1b[38;5;208melse\x1b[0m {\n\
\x1b[38;5;240m  15\x1b[0m         \x1b[38;5;220mBox\x1b[0m::new(CliTmuxClient::new())\n\
\x1b[38;5;240m  16\x1b[0m     };\n\
\x1b[38;5;240m  17\x1b[0m \n\
\x1b[38;5;240m  18\x1b[0m     \x1b[38;5;208mlet mut\x1b[0m app = App::new(client, config, is_mock);\n\
\x1b[38;5;240m  19\x1b[0m     app.run()?;\n\
\x1b[38;5;240m  20\x1b[0m     \x1b[38;5;220mOk\x1b[0m(())\n\
\x1b[38;5;240m  21\x1b[0m }\n\
\x1b[38;5;240m  22\x1b[0m \n\
\x1b[38;5;240m  23\x1b[0m \x1b[38;5;246m// NORMAL \x1b[38;5;39msrc/main.rs\x1b[0m \x1b[38;5;240mutf-8[unix] \x1b[38;5;120m21:1\x1b[0m"
                .to_vec(),
        );

        let mut p2 = Pane::new(
            PaneId::from("%2"),
            w1_id.clone(),
            s1_id.clone(),
            2,
            false,
            "cargo test --watch".to_string(),
            PathBuf::from("~/github.com/amjadjibon/lazytmux"),
            80,
            50,
        );
        p2.git_branch = Some("main".to_string());
        p2.set_preview(
            b"\x1b[38;5;245m$ cargo test --all-targets\x1b[0m\n\
   \x1b[32;1mCompiling\x1b[0m lazytmux v0.1.2 (/Users/amjad/lazytmux)\n\
    \x1b[32;1mFinished\x1b[0m `test` profile [unoptimized + debuginfo] in 0.82s\n\
     \x1b[32;1mRunning\x1b[0m unittests src/lib.rs\n\
\n\
running 25 tests\n\
test domain::layout::tests::test_dimensions ... \x1b[32mok\x1b[0m\n\
test domain::layout::tests::test_parse_nested ... \x1b[32mok\x1b[0m\n\
test domain::sanitizer::tests::test_dangerous_chars ... \x1b[32mok\x1b[0m\n\
test domain::pane::tests::test_detect_git_branch ... \x1b[32mok\x1b[0m\n\
test tmux::parser::tests::test_parse_sessions ... \x1b[32mok\x1b[0m\n\
test tmux::parser::tests::test_parse_windows ... \x1b[32mok\x1b[0m\n\
test tmux::parser::tests::test_assemble_tree ... \x1b[32mok\x1b[0m\n\
test ui::layout::tests::test_split ... \x1b[32mok\x1b[0m\n\
\n\
\x1b[38;5;245m[INFO] Watching for file changes in src/...\x1b[0m\n\
\x1b[32;1mtest result: ok.\x1b[0m 25 passed; 0 failed; 0 ignored; 0 measured"
                .to_vec(),
        );

        w1.panes = vec![p1, p2];
        w1.layout_str = generate_layout_string(&w1.panes, "even-horizontal");

        // Window 1.2: "backend" (3 panes: Next.js + Docker Compose + Lazygit)
        let w2_id = WindowId::from("@2");
        let mut w2 = Window::new(
            w2_id.clone(),
            s1_id.clone(),
            2,
            "backend".to_string(),
            false,
            "".to_string(),
        );

        let mut p3 = Pane::new(
            PaneId::from("%3"),
            w2_id.clone(),
            s1_id.clone(),
            1,
            true,
            "npm run dev".to_string(),
            PathBuf::from("~/code/frontend"),
            100,
            50,
        );
        p3.git_branch = Some("feature/auth".to_string());
        p3.set_preview(
            b"\x1b[36mready\x1b[0m - started server on 0.0.0.0:3000, url: \x1b[34mhttp://localhost:3000\x1b[0m\n\
\x1b[32m[INFO]\x1b[0m 19:40:02 compiled client and server successfully in 312 ms (892 modules)\n\
\x1b[32m[INFO]\x1b[0m 19:40:15 GET /api/v1/auth/session \x1b[32m200 OK\x1b[0m in 14ms\n\
\x1b[32m[INFO]\x1b[0m 19:40:18 GET /dashboard \x1b[32m200 OK\x1b[0m in 32ms\n\
\x1b[33m[WARN]\x1b[0m 19:40:22 [Fast Refresh] rebuilding components/Header.tsx\n\
\x1b[32m[INFO]\x1b[0m 19:40:23 compiled client and server successfully in 84 ms\n\
\x1b[32m[INFO]\x1b[0m 19:40:35 POST /api/v1/workspaces/create \x1b[32m201 Created\x1b[0m in 45ms\n\
\x1b[32m[INFO]\x1b[0m 19:40:40 GET /api/v1/metrics \x1b[32m200 OK\x1b[0m in 2ms"
                .to_vec(),
        );

        let mut p4 = Pane::new(
            PaneId::from("%4"),
            w2_id.clone(),
            s1_id.clone(),
            2,
            false,
            "docker compose logs -f".to_string(),
            PathBuf::from("~/code/infra"),
            60,
            25,
        );
        p4.git_branch = Some("dev".to_string());
        p4.set_preview(
            b"\x1b[36mpostgres-1  |\x1b[0m PostgreSQL Database directory appears to contain a database; Skipping initialization\n\
\x1b[36mpostgres-1  |\x1b[0m 2026-09-02 19:30:00.120 UTC [1] LOG:  database system was shut down at 2026-09-02 19:28:44 UTC\n\
\x1b[36mpostgres-1  |\x1b[0m 2026-09-02 19:30:00.145 UTC [1] LOG:  database system is ready to accept connections\n\
\x1b[35mredis-1     |\x1b[0m 1:M 02 Sep 2026 19:30:00.200 * Running mode=standalone, port=6379.\n\
\x1b[35mredis-1     |\x1b[0m 1:M 02 Sep 2026 19:30:00.201 * Ready to accept connections tcp\n\
\x1b[33mrabbitmq-1  |\x1b[0m 2026-09-02 19:30:01.400 [info] Server startup complete; 4 plugins started."
                .to_vec(),
        );

        let mut p5 = Pane::new(
            PaneId::from("%5"),
            w2_id.clone(),
            s1_id.clone(),
            3,
            false,
            "lazygit".to_string(),
            PathBuf::from("~/code/frontend"),
            60,
            25,
        );
        p5.git_branch = Some("feature/auth".to_string());
        p5.set_preview(
            b"\x1b[38;5;220mcommit 4a9f1b2 (HEAD -> feature/auth, origin/feature/auth)\x1b[0m\n\
Author: Amjad Hossain <amjad.jibon@gmail.com>\n\
Date:   Wed Sep 2 19:35:00 2026 +0800\n\
\n\
    feat(auth): integrate OAuth2 session verification and token refresh\n\
\n\
\x1b[32m+   pub async fn verify_session(token: &str) -> Result<UserSession>\x1b[0m\n\
\x1b[32m+   pub async fn refresh_credentials(refresh_token: &str) -> Result<()>\x1b[0m\n\
\x1b[31m-   // legacy session check\x1b[0m"
                .to_vec(),
        );

        w2.panes = vec![p3, p4, p5];
        w2.layout_str = generate_layout_string(&w2.panes, "main-vertical");

        // Window 1.3: "monitor" (2 panes: htop + system logs)
        let w3_id = WindowId::from("@3");
        let mut w3 = Window::new(
            w3_id.clone(),
            s1_id.clone(),
            3,
            "monitor".to_string(),
            false,
            "".to_string(),
        );

        let mut p6 = Pane::new(
            PaneId::from("%6"),
            w3_id.clone(),
            s1_id.clone(),
            1,
            true,
            "htop".to_string(),
            PathBuf::from("~"),
            160,
            25,
        );
        p6.set_preview(
            b"\x1b[32m1  [|||||||||||||||||||||                     38.2%]\x1b[0m   Tasks: \x1b[32m318\x1b[0m, 1 thr; \x1b[32m1 running\x1b[0m\n\
\x1b[32m2  [||||||||||                                 18.5%]\x1b[0m   Load average: \x1b[32m1.45 1.62 1.80\x1b[0m\n\
\x1b[32m3  [||||||||||||||||                           29.0%]\x1b[0m   Uptime: \x1b[32m14 days, 06:12:40\x1b[0m\n\
\x1b[34mMem[|||||||||||||||||||||||||||||||||    12.4G/32.0G]\x1b[0m\n\
\x1b[34mSwp[                                        0K/8.00G]\x1b[0m\n\
\n\
\x1b[7m  PID USER      PRI  NI  VIRT   RES   SHR S CPU% MEM%   TIME+  Command                     \x1b[0m\n\
 4812 amjad      20   0 1482M  180M  4200 S 14.2  0.6  0:12.40 lazytmux\n\
 3910 amjad      20   0 4210M  820M 12400 S  8.4  2.5  1:45.12 nvim src/main.rs\n\
 2190 amjad      20   0 2190M  340M  6800 S  4.1  1.0  0:48.09 cargo test\n\
 1820 amjad      20   0 3100M  480M  9100 S  2.2  1.5  2:10.15 node server.js"
                .to_vec(),
        );

        let mut p7 = Pane::new(
            PaneId::from("%7"),
            w3_id.clone(),
            s1_id.clone(),
            2,
            false,
            "tail -f /var/log/system.log".to_string(),
            PathBuf::from("/var/log"),
            160,
            25,
        );
        p7.set_preview(
            b"Sep  2 19:20:00 macbook syslogd[1]: restart\n\
Sep  2 19:22:15 macbook kernel[0]: [wlan0] associated with AP 5g-office (BSSID 00:11:22:33:44:55)\n\
Sep  2 19:25:40 macbook tmux[4120]: session 'lazytmux' attached\n\
Sep  2 19:28:10 macbook lazytmux[4812]: initialized 3-column TUI layout with TokyoNight theme\n\
Sep  2 19:30:00 macbook kernel[0]: disk0s2: verified APFS volume clean"
                .to_vec(),
        );

        w3.panes = vec![p6, p7];
        w3.layout_str = generate_layout_string(&w3.panes, "even-vertical");

        // Window 1.4: "shell" (1 pane: zsh)
        let w4_id = WindowId::from("@4");
        let mut w4 = Window::new(
            w4_id.clone(),
            s1_id.clone(),
            4,
            "shell".to_string(),
            false,
            "".to_string(),
        );
        let mut p8 = Pane::new(
            PaneId::from("%8"),
            w4_id.clone(),
            s1_id.clone(),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("~/github.com/amjadjibon/lazytmux"),
            160,
            50,
        );
        p8.git_branch = Some("main".to_string());
        p8.set_preview(
            b"\x1b[32mamjad@macbook\x1b[0m:\x1b[34m~/github.com/amjadjibon/lazytmux\x1b[0m (main)\n\
$ git status\n\
On branch main\n\
Your branch is up to date with 'origin/main'.\n\
\n\
nothing to commit, working tree clean\n\
$ "
            .to_vec(),
        );
        w4.panes = vec![p8];
        w4.layout_str = generate_layout_string(&w4.panes, "even-horizontal");

        s1.windows = vec![w1, w2, w3, w4];

        // -------------------------------------------------------------
        // Session 2: "personal"
        // -------------------------------------------------------------
        let s2_id = SessionId::from("$2");
        let mut s2 = Session::new(s2_id.clone(), "personal".to_string(), 2, false);

        let w5_id = WindowId::from("@5");
        let mut w5 = Window::new(
            w5_id.clone(),
            s2_id.clone(),
            1,
            "blog".to_string(),
            true,
            "".to_string(),
        );
        let mut p9 = Pane::new(
            PaneId::from("%9"),
            w5_id.clone(),
            s2_id.clone(),
            1,
            true,
            "hugo server".to_string(),
            PathBuf::from("~/personal/blog"),
            160,
            50,
        );
        p9.git_branch = Some("main".to_string());
        p9.set_preview(
            b"Watching for config changes in ~/personal/blog/hugo.toml\n\
\x1b[32mWeb Server is available at http://localhost:1313/\x1b[0m (bind address 127.0.0.1)\n\
Total in 42 ms\n\
Press Ctrl+C to stop"
                .to_vec(),
        );
        w5.panes = vec![p9];
        w5.layout_str = generate_layout_string(&w5.panes, "even-horizontal");

        let w6_id = WindowId::from("@6");
        let mut w6 = Window::new(
            w6_id.clone(),
            s2_id.clone(),
            2,
            "notes".to_string(),
            false,
            "".to_string(),
        );
        let mut p10 = Pane::new(
            PaneId::from("%10"),
            w6_id.clone(),
            s2_id.clone(),
            1,
            true,
            "nvim notes.md".to_string(),
            PathBuf::from("~/personal"),
            160,
            50,
        );
        p10.set_preview(
            b"# Personal Notes\n\
- [x] Implement lazytmux live layout switching\n\
- [x] Add in-buffer scrollback search\n\
- [ ] Add mouse drag resizing support"
                .to_vec(),
        );
        w6.panes = vec![p10];
        w6.layout_str = generate_layout_string(&w6.panes, "even-horizontal");
        s2.windows = vec![w5, w6];

        // -------------------------------------------------------------
        // Session 3: "infra"
        // -------------------------------------------------------------
        let s3_id = SessionId::from("$3");
        let mut s3 = Session::new(s3_id.clone(), "infra".to_string(), 1, false);

        let w7_id = WindowId::from("@7");
        let mut w7 = Window::new(
            w7_id.clone(),
            s3_id.clone(),
            1,
            "terraform".to_string(),
            true,
            "".to_string(),
        );
        let mut p11 = Pane::new(
            PaneId::from("%11"),
            w7_id.clone(),
            s3_id.clone(),
            1,
            true,
            "terraform plan".to_string(),
            PathBuf::from("~/infra/terraform"),
            80,
            50,
        );
        p11.git_branch = Some("main".to_string());
        p11.set_preview(
            b"\x1b[32mTerraform will perform the following actions:\x1b[0m\n\n\
  \x1b[32m# aws_s3_bucket.artifacts will be created\x1b[0m\n\
  \x1b[32m+\x1b[0m resource \"aws_s3_bucket\" \"artifacts\" {\n\
      \x1b[32m+\x1b[0m bucket = \"lazytmux-build-artifacts\"\n\
      \x1b[32m+\x1b[0m region = \"us-east-1\"\n\
    }\n\n\
\x1b[32mPlan:\x1b[0m 1 to add, 0 to change, 0 to destroy."
                .to_vec(),
        );

        let mut p12 = Pane::new(
            PaneId::from("%12"),
            w7_id.clone(),
            s3_id.clone(),
            2,
            false,
            "k9s".to_string(),
            PathBuf::from("~/infra/k8s"),
            80,
            50,
        );
        p12.git_branch = Some("infra-prod".to_string());
        p12.set_preview(
            b"\x1b[36mContext: prod-us-east-1\x1b[0m \x1b[33mCluster: k8s-main\x1b[0m\n\
\x1b[7m  NAMESPACE   NAME                        READY   STATUS    AGE   \x1b[0m\n\
  production  api-deployment-78b9d4-1     1/1     \x1b[32mRunning\x1b[0m   12d\n\
  production  web-frontend-54c2a1-1       1/1     \x1b[32mRunning\x1b[0m   4d"
                .to_vec(),
        );
        w7.panes = vec![p11, p12];
        w7.layout_str = generate_layout_string(&w7.panes, "even-horizontal");
        s3.windows = vec![w7];

        self.sessions = vec![s1, s2, s3];
    }
}

fn pane_id_num(pane: &Pane) -> String {
    let s = pane.id.0.trim_start_matches('%');
    if s.is_empty() {
        pane.index.to_string()
    } else {
        s.to_string()
    }
}

pub fn generate_layout_string(panes: &[Pane], layout: &str) -> String {
    let count = panes.len();
    if count == 0 {
        return "0000,160x50,0,0,0".to_string();
    }
    if count == 1 {
        return format!("0000,160x50,0,0,{}", pane_id_num(&panes[0]));
    }

    match layout {
        "even-horizontal" => {
            let width_per_pane = (160 / count as u16).max(1);
            let children = panes
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let x = i as u16 * width_per_pane;
                    format!("{width_per_pane}x50,{x},0,{}", pane_id_num(p))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("0000,160x50,0,0{{{children}}}")
        }
        "even-vertical" => {
            let height_per_pane = (50 / count as u16).max(1);
            let children = panes
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let y = i as u16 * height_per_pane;
                    format!("160x{height_per_pane},0,{y},{}", pane_id_num(p))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("0000,160x50,0,0[{children}]")
        }
        "main-horizontal" => {
            let top_h = 30u16;
            let bot_h = 20u16;
            let sub_count = (count - 1).max(1);
            let sub_w = 160 / sub_count as u16;
            let sub_panes = panes[1..]
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let x = i as u16 * sub_w;
                    format!("{sub_w}x{bot_h},{x},{top_h},{}", pane_id_num(p))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "0000,160x50,0,0[160x{top_h},0,0,{},160x{bot_h},0,{top_h}{{{sub_panes}}}]",
                pane_id_num(&panes[0])
            )
        }
        "main-vertical" => {
            let main_w = 100u16;
            let side_w = 60u16;
            let sub_count = (count - 1).max(1);
            let sub_h = 50 / sub_count as u16;
            let sub_panes = panes[1..]
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let y = i as u16 * sub_h;
                    format!("{side_w}x{sub_h},{main_w},{y},{}", pane_id_num(p))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "0000,160x50,0,0{{{main_w}x50,0,0,{},{side_w}x50,{main_w},0[{sub_panes}]}}",
                pane_id_num(&panes[0])
            )
        }
        _ => {
            // tiled or default
            let width_per_pane = (160 / count as u16).max(1);
            let children = panes
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let x = i as u16 * width_per_pane;
                    format!("{width_per_pane}x50,{x},0,{}", pane_id_num(p))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("0000,160x50,0,0{{{children}}}")
        }
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
                    let text = p.preview_lines.join("\n");
                    return Ok(text.into_bytes());
                }
            }
        }
        Ok(b"No preview available".to_vec())
    }

    fn create_session(&mut self, name: &str) -> Result<SessionId> {
        self.counter += 1;
        let new_id = SessionId(format!("${}", self.counter));
        let mut session = Session::new(new_id.clone(), name.to_string(), 1, false);

        let win_id = WindowId(format!("@{}", self.counter * 10));
        let mut window = Window::new(
            win_id.clone(),
            new_id.clone(),
            1,
            "main".to_string(),
            true,
            "0000,160x50,0,0,1".to_string(),
        );

        let mut pane = Pane::new(
            PaneId(format!("%{}", self.counter * 100)),
            win_id,
            new_id.clone(),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("~/workspace"),
            160,
            50,
        );
        pane.git_branch = Some("main".to_string());
        pane.set_preview(b"Terminal initialized.\n$ ".to_vec());
        window.panes = vec![pane];
        session.windows = vec![window];

        self.sessions.push(session);
        Ok(new_id)
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
        let new_id = WindowId(format!("@{}", self.counter));
        if let Some(s) = self.sessions.iter_mut().find(|s| &s.id == session) {
            let next_idx = s.windows.len() as u32 + 1;
            let mut window = Window::new(
                new_id.clone(),
                session.clone(),
                next_idx,
                name.to_string(),
                false,
                "0000,160x50,0,0,1".to_string(),
            );
            let mut pane = Pane::new(
                PaneId(format!("%{}", self.counter * 100)),
                new_id.clone(),
                session.clone(),
                1,
                true,
                "zsh".to_string(),
                PathBuf::from("~/workspace"),
                160,
                50,
            );
            pane.git_branch = Some("main".to_string());
            pane.set_preview(b"Window terminal ready.\n$ ".to_vec());
            window.panes = vec![pane];
            s.windows.push(window);
            s.window_count = s.windows.len();
            Ok(new_id)
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
            let initial_len = s.windows.len();
            s.windows.retain(|w| &w.id != window);
            if s.windows.len() < initial_len {
                s.window_count = s.windows.len();
                return Ok(());
            }
        }
        Err(anyhow!("Window not found"))
    }

    fn kill_pane(&mut self, pane: &PaneId) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                let initial_len = w.panes.len();
                w.panes.retain(|p| &p.id != pane);
                if w.panes.len() < initial_len {
                    w.layout_str = generate_layout_string(&w.panes, "even-horizontal");
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Pane not found"))
    }

    fn zoom_pane(&mut self, pane: &PaneId) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(p) = w.panes.iter_mut().find(|p| &p.id == pane) {
                    p.active = !p.active;
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Pane not found"))
    }

    fn split_pane(&mut self, pane: &PaneId, vertical: bool) -> Result<PaneId> {
        self.counter += 1;
        let new_pane_id = PaneId(format!("%{}", self.counter));
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if w.panes.iter().any(|p| &p.id == pane) {
                    let mut new_pane = Pane::new(
                        new_pane_id.clone(),
                        w.id.clone(),
                        s.id.clone(),
                        w.panes.len() as u32 + 1,
                        false,
                        "zsh".to_string(),
                        PathBuf::from("~/workspace"),
                        80,
                        25,
                    );
                    new_pane.git_branch = Some("main".to_string());
                    new_pane.set_preview(b"New terminal pane initialized.\n$ ".to_vec());
                    w.panes.push(new_pane);
                    let layout_type = if vertical {
                        "even-vertical"
                    } else {
                        "even-horizontal"
                    };
                    w.layout_str = generate_layout_string(&w.panes, layout_type);
                    return Ok(new_pane_id);
                }
            }
        }
        Err(anyhow!("Target pane not found"))
    }

    fn select_layout(&mut self, window: &WindowId, layout: &str) -> Result<()> {
        for s in &mut self.sessions {
            if let Some(w) = s.windows.iter_mut().find(|w| &w.id == window) {
                w.layout_str = generate_layout_string(&w.panes, layout);
                return Ok(());
            }
        }
        Ok(())
    }

    fn toggle_sync_panes(&mut self, window: &WindowId) -> Result<bool> {
        if self.synced_windows.contains(window) {
            self.synced_windows.remove(window);
            Ok(false)
        } else {
            self.synced_windows.insert(window.clone());
            Ok(true)
        }
    }

    fn swap_pane(&mut self, pane: &PaneId, up: bool) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(idx) = w.panes.iter().position(|p| &p.id == pane) {
                    if up && idx > 0 {
                        w.panes.swap(idx, idx - 1);
                    } else if !up && idx + 1 < w.panes.len() {
                        w.panes.swap(idx, idx + 1);
                    }
                    w.layout_str = generate_layout_string(&w.panes, "even-horizontal");
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn swap_window(&mut self, window: &WindowId, left: bool) -> Result<()> {
        for s in &mut self.sessions {
            if let Some(idx) = s.windows.iter().position(|w| &w.id == window) {
                if left && idx > 0 {
                    s.windows.swap(idx, idx - 1);
                } else if !left && idx + 1 < s.windows.len() {
                    s.windows.swap(idx, idx + 1);
                }
                return Ok(());
            }
        }
        Ok(())
    }

    fn respawn_pane(&mut self, pane: &PaneId) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(p) = w.panes.iter_mut().find(|p| &p.id == pane) {
                    p.preview_lines
                        .push("\x1b[33m[Process respawned by lazytmux]\x1b[0m".to_string());
                    p.preview_lines.push("$ zsh".to_string());
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn send_keys(&mut self, pane: &PaneId, keys: &str) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(p) = w.panes.iter_mut().find(|p| &p.id == pane) {
                    p.preview_lines.push(format!("$ {keys}"));
                    p.preview_lines.push("Execution complete.".to_string());
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn send_keys_with_enter(&mut self, pane: &PaneId, keys: &str) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(p) = w.panes.iter_mut().find(|p| &p.id == pane) {
                    if keys.is_empty() {
                        p.preview_lines.push("<Enter>".to_string());
                    } else {
                        p.preview_lines.push(format!("$ {keys} + <Enter>"));
                        p.preview_lines.push("Execution complete.".to_string());
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn break_pane(&mut self, pane: &PaneId) -> Result<()> {
        for s in &mut self.sessions {
            for w_idx in 0..s.windows.len() {
                if let Some(p_idx) = s.windows[w_idx].panes.iter().position(|p| &p.id == pane) {
                    let mut broken_pane = s.windows[w_idx].panes.remove(p_idx);
                    s.windows[w_idx].layout_str =
                        generate_layout_string(&s.windows[w_idx].panes, "even-horizontal");
                    self.counter += 1;
                    let new_win_id = WindowId(format!("@{}", self.counter * 10));
                    broken_pane.window_id = new_win_id.clone();
                    let mut new_win = Window::new(
                        new_win_id,
                        s.id.clone(),
                        s.windows.len() as u32 + 1,
                        broken_pane.current_command.clone(),
                        true,
                        "0000,160x50,0,0,1".to_string(),
                    );
                    new_win.panes.push(broken_pane);
                    s.windows.push(new_win);
                    s.window_count = s.windows.len();
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn resize_pane(
        &mut self,
        pane: &PaneId,
        direction: ResizeDirection,
        amount: usize,
    ) -> Result<()> {
        for s in &mut self.sessions {
            for w in &mut s.windows {
                if let Some(p) = w.panes.iter_mut().find(|p| &p.id == pane) {
                    match direction {
                        ResizeDirection::Up => {
                            p.height = p.height.saturating_sub(amount as u16).max(5);
                        }
                        ResizeDirection::Down => {
                            p.height = (p.height + amount as u16).min(100);
                        }
                        ResizeDirection::Left => {
                            p.width = p.width.saturating_sub(amount as u16).max(10);
                        }
                        ResizeDirection::Right => {
                            p.width = (p.width + amount as u16).min(200);
                        }
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn focus_pane(&self, _session: &SessionId, _window: &WindowId, _pane: &PaneId) -> Result<()> {
        Ok(())
    }
}
