use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
    running: Arc<AtomicBool>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));

        let tx_events = tx.clone();
        let running_events = running.clone();

        // Crossterm event polling thread
        thread::spawn(move || {
            while running_events.load(Ordering::Relaxed) {
                if let Ok(true) = event::poll(Duration::from_millis(50)) {
                    let app_event = match event::read() {
                        Ok(CrosstermEvent::Key(key)) => Some(AppEvent::Key(key)),
                        Ok(CrosstermEvent::Mouse(mouse)) => Some(AppEvent::Mouse(mouse)),
                        Ok(CrosstermEvent::Resize(w, h)) => Some(AppEvent::Resize(w, h)),
                        _ => None,
                    };

                    if let Some(ev) = app_event
                        && tx_events.send(ev).is_err()
                    {
                        break;
                    }
                }
            }
        });

        // Periodic tick thread
        let running_tick = running.clone();
        thread::spawn(move || {
            let tick_interval = Duration::from_millis(tick_rate_ms);
            while running_tick.load(Ordering::Relaxed) {
                thread::sleep(tick_interval);
                if tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        Self { rx, running }
    }

    pub fn next(&self) -> Result<AppEvent, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self.stop();
    }
}
