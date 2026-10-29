// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Kernel: resolves the dependency graph, boots every component, runs lifecycle hooks.

use std::path::{Path, PathBuf};

use super::container::Container;
use super::error::Result;
use super::events::EventBus;
use super::modules::{load_manifest, resolve};
use super::security::integrity;
use super::services::GreetingService;
use super::values::Value;

const LOG: &str = "core.bootstrap";

pub fn default_manifest() -> PathBuf {
    super::package_dir().join("manifests").join("services.json")
}

pub struct Kernel {
    pub container: Container,
}

impl Kernel {
    pub fn boot() -> Result<Kernel> {
        Self::boot_from(&default_manifest())
    }

    pub fn boot_from(manifest: &Path) -> Result<Kernel> {
        integrity::verify()?;
        let order = resolve(&load_manifest(manifest)?)?;
        crate::log_info!(LOG, "boot order: {}", order.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(" -> "));
        let mut container = Container::new();
        for component in &order {
            let instance = container.instantiate(component)?;
            instance.on_start()?;
        }
        container.get::<EventBus>("event_bus")?.publish("kernel.booted", Value::Int(order.len() as i64))?;
        Ok(Kernel { container })
    }

    pub fn produce_greeting(&self) -> Result<String> {
        self.container.get::<GreetingService>("greeting_service")?.produce()
    }

    pub fn render(&self, message: &str) -> Result<()> {
        self.container.get::<GreetingService>("greeting_service")?.deliver(message)
    }
}
