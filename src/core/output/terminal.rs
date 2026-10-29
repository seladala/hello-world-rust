// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Terminal Rendering Engine: turns approved glyphs into photons (via stdout).

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::events::{Event, EventBus};
use crate::core::values::Value;

#[derive(Default)]
pub struct GlyphBuffer {
    glyphs: Vec<String>,
}

impl GlyphBuffer {
    pub fn push(&mut self, glyph: &str) {
        self.glyphs.push(glyph.to_owned());
    }

    pub fn drain(&mut self) -> String {
        std::mem::take(&mut self.glyphs).concat()
    }
}

pub struct TerminalRenderingEngine {
    newline: bool,
    buffer: RefCell<GlyphBuffer>,
}

impl Service for TerminalRenderingEngine {}

impl TerminalRenderingEngine {
    pub fn new(config: &EnterpriseConfiguration, event_bus: &EventBus) -> Result<Rc<Self>> {
        let engine = Rc::new(TerminalRenderingEngine {
            newline: config.get_bool("rendering", "trailing_newline")?,
            buffer: RefCell::new(GlyphBuffer::default()),
        });
        let weak = Rc::downgrade(&engine);
        event_bus.subscribe(
            "glyph.render",
            Rc::new(move |event: &Event| weak.upgrade().map_or(Ok(()), |engine| engine.on_glyph(event))),
        );
        let weak = Rc::downgrade(&engine);
        event_bus.subscribe(
            "transmission.complete",
            Rc::new(move |event: &Event| weak.upgrade().map_or(Ok(()), |engine| engine.on_complete(event))),
        );
        Ok(engine)
    }

    pub fn on_glyph(&self, event: &Event) -> Result<()> {
        let glyph = event.payload.as_str().ok_or_else(|| error::type_error("glyphs must be strings"))?;
        self.buffer.borrow_mut().push(glyph);
        Ok(())
    }

    pub fn on_complete(&self, event: &Event) -> Result<()> {
        let text = self.buffer.borrow_mut().drain();
        if Value::Int(text.chars().count() as i64) != event.payload {
            return Err(error::runtime_error("glyphs were lost in transit"));
        }
        let mut stdout = std::io::stdout().lock();
        let output = if self.newline { format!("{text}\n") } else { text };
        stdout.write_all(output.as_bytes()).and_then(|()| stdout.flush()).map_err(|e| error::os_error(e.to_string()))
    }
}
