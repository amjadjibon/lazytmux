use crate::domain::{PaneId, Session};
use crate::event::AppEvent;
use crate::tmux::{CliTmuxClient, TmuxClient};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

/// What the UI currently has on screen, so the poller knows which panes are
/// worth capturing and how deeply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewContext {
    /// Panes of the visible window, in display order.
    pub panes: Vec<PaneId>,
    /// The pane open in Inspect mode, which needs the deep buffer.
    pub inspected: Option<PaneId>,
    pub preview_lines: usize,
    pub inspect_lines: usize,
}

enum PollRequest {
    /// The visible window changed.
    Context(Box<PreviewContext>),
    /// Something happened that invalidates the current tree; poll now.
    Now,
}

/// Control channel to a running [`spawn`]ed poller. Dropping it stops the thread.
#[derive(Debug)]
pub struct PollerHandle {
    tx: Sender<PollRequest>,
}

impl PollerHandle {
    /// Tell the poller what is on screen and refresh immediately.
    pub fn set_context(&self, context: PreviewContext) {
        let _ = self.tx.send(PollRequest::Context(Box::new(context)));
    }

    /// Refresh as soon as possible without changing the context.
    pub fn refresh_now(&self) {
        let _ = self.tx.send(PollRequest::Now);
    }
}

/// Run tmux queries on a background thread, publishing each result as
/// [`AppEvent::Data`].
///
/// Every `tmux` invocation costs milliseconds, which is far more than this
/// program spends computing. Doing that work on the UI thread meant input was
/// blocked for the duration of every poll, and a tmux server that stopped
/// answering froze the interface outright. Here a stall costs only staleness:
/// the UI keeps drawing and stays quittable.
///
/// The poller owns its own client, which is why [`CliTmuxClient`] is a unit
/// struct — there is no state to share, so no lock is needed.
pub fn spawn(interval: Duration, events: Sender<AppEvent>) -> PollerHandle {
    let (tx, rx) = mpsc::channel::<PollRequest>();

    thread::spawn(move || {
        let client = CliTmuxClient::new();
        let mut context = PreviewContext::default();

        loop {
            if let Ok(mut tree) = client.fetch_full_tree() {
                fill_previews(&client, &mut tree, &context);
                if events.send(AppEvent::Data(tree)).is_err() {
                    // The UI is gone.
                    break;
                }
            }

            match rx.recv_timeout(interval) {
                Ok(request) => {
                    apply(&mut context, request);
                    // Collapse a burst — holding j scrolls through many windows
                    // — into a single refresh.
                    while let Ok(request) = rx.try_recv() {
                        apply(&mut context, request);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    PollerHandle { tx }
}

fn apply(context: &mut PreviewContext, request: PollRequest) {
    if let PollRequest::Context(new) = request {
        *context = *new;
    }
}

/// Capture previews for the panes the UI is showing, in as few `tmux`
/// invocations as possible: one per distinct capture depth.
fn fill_previews(client: &dyn TmuxClient, tree: &mut [Session], context: &PreviewContext) {
    if context.panes.is_empty() {
        return;
    }

    let (deep, shallow): (Vec<PaneId>, Vec<PaneId>) = context
        .panes
        .iter()
        .cloned()
        .partition(|id| context.inspected.as_ref() == Some(id));

    for (ids, depth) in [
        (shallow, context.preview_lines),
        (deep, context.inspect_lines),
    ] {
        if ids.is_empty() {
            continue;
        }
        for (id, captured) in ids.iter().zip(client.capture_panes(&ids, depth, true)) {
            if let Some(raw) = captured
                && let Some(pane) = find_pane(tree, id)
            {
                pane.set_preview(raw);
            }
        }
    }
}

fn find_pane<'a>(tree: &'a mut [Session], id: &PaneId) -> Option<&'a mut crate::domain::Pane> {
    tree.iter_mut()
        .flat_map(|s| s.windows.iter_mut())
        .flat_map(|w| w.panes.iter_mut())
        .find(|p| &p.id == id)
}
