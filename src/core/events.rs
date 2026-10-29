//! Synchronous, in-process, single-threaded, enterprise-grade event bus.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::container::Service;
use super::error::Result;
use super::values::Value;

const LOG: &str = "core.events";

static SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub topic: String,
    pub payload: Value,
    pub sequence: u64,
}

impl Event {
    pub fn new(topic: &str, payload: Value) -> Event {
        Event { topic: topic.to_owned(), payload, sequence: SEQUENCE.fetch_add(1, Ordering::Relaxed) }
    }
}

pub type Handler = Rc<dyn Fn(&Event) -> Result<()>>;

#[derive(Default)]
pub struct EventBus {
    subscribers: RefCell<HashMap<String, Vec<Handler>>>,
    pub history: RefCell<Vec<Event>>,
}

impl Service for EventBus {}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, topic: &str, handler: Handler) {
        self.subscribers.borrow_mut().entry(topic.to_owned()).or_default().push(handler);
    }

    pub fn publish(&self, topic: &str, payload: Value) -> Result<Event> {
        let event = Event::new(topic, payload);
        self.history.borrow_mut().push(event.clone());
        crate::log_debug!(LOG, "event #{:05} {:<28} {}", event.sequence, topic, event.payload.repr());
        for wanted in [topic, "*"] {
            let handlers: Vec<Handler> = self.subscribers.borrow().get(wanted).cloned().unwrap_or_default();
            for handler in handlers {
                handler(&event)?;
            }
        }
        Ok(event)
    }
}
