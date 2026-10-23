use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

pub enum Event {
    Tick,
    Key(KeyEvent),
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> anyhow::Result<Event> {
        if event::poll(self.tick_rate)? {
            if let CrosstermEvent::Key(key) = event::read()? {
                return Ok(Event::Key(key));
            }
        }
        Ok(Event::Tick)
    }
}
