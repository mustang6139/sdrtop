use crossterm::event::{self, Event, KeyEvent};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub struct EventStream {
    rx: Receiver<AppEvent>,
}

impl EventStream {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
            } else if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        });
        Self { rx }
    }

    pub fn recv(&self) -> AppEvent {
        self.rx.recv().unwrap_or(AppEvent::Tick)
    }
}
