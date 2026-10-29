//! Inversion-of-Control container.
//!
//! Components are instantiated in dependency order; each provider receives its
//! dependencies as keyword arguments named after the components it requires.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use super::config::EnterpriseConfiguration;
use super::diagnostics::entropy::EntropyPool;
use super::diagnostics::health_check::WarmupOrchestrator;
use super::error::{self, Result};
use super::events::EventBus;
use super::modules::Component;
use super::output::terminal::TerminalRenderingEngine;
use super::security::anticheat::AntiCheatEngine;
use super::security::drm::LicenseManager;
use super::services::{CpuService, FirmwareLoader, GreetingService, PiService};
use super::validation::audit_log::GlyphChain;
use super::validation::normalization::LetterCaseNegotiator;
use super::validation::policy::GlyphApprovalCommittee;
use super::values::repr_str;

const LOG: &str = "core.container";

pub trait AsAny {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any>;
}

impl<T: Any> AsAny for T {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// Anything the container can run. `on_start` is the optional lifecycle hook.
pub trait Service: AsAny + 'static {
    fn on_start(&self) -> Result<()> {
        Ok(())
    }
}

pub type Instance = Rc<dyn Service>;

fn downcast<T: Service>(name: &str, instance: &Instance) -> Result<Rc<T>> {
    instance
        .clone()
        .into_any()
        .downcast::<T>()
        .map_err(|_| error::type_error(format!("component {} is not a {}", repr_str(name), std::any::type_name::<T>())))
}

/// Keyword arguments handed to a provider.
pub struct Kwargs {
    values: Vec<(String, Instance)>,
}

impl Kwargs {
    pub fn get<T: Service>(&self, name: &str) -> Result<Rc<T>> {
        let (_, instance) = self
            .values
            .iter()
            .find(|(key, _)| key == name)
            .ok_or_else(|| error::type_error(format!("missing required keyword argument: {}", repr_str(name))))?;
        downcast(name, instance)
    }
}

pub type Constructor = fn(&Kwargs) -> Result<Instance>;

/// A provider: the constructor behind a `module:Attribute` path and the parameters it accepts.
pub struct Provider {
    pub path: &'static str,
    pub params: &'static [&'static str],
    pub construct: Constructor,
}

impl Provider {
    fn qualname(&self) -> &'static str {
        self.path.rsplit(':').next().unwrap_or(self.path)
    }
}

fn wrap<T: Service>(result: Result<Rc<T>>) -> Result<Instance> {
    result.map(|instance| instance as Instance)
}

pub static PROVIDERS: &[Provider] = &[
    Provider {
        path: "core.config:EnterpriseConfiguration",
        params: &[],
        construct: |_| wrap(EnterpriseConfiguration::new().map(Rc::new)),
    },
    Provider { path: "core.events:EventBus", params: &[], construct: |_| wrap(Ok(Rc::new(EventBus::new()))) },
    Provider {
        path: "core.diagnostics.entropy:EntropyPool",
        params: &["config"],
        construct: |kw| wrap(EntropyPool::new(&*kw.get::<EnterpriseConfiguration>("config")?).map(Rc::new)),
    },
    Provider {
        path: "core.diagnostics.health_check:WarmupOrchestrator",
        params: &["config", "entropy_pool"],
        construct: |kw| wrap(Ok(Rc::new(WarmupOrchestrator::new(kw.get("config")?, kw.get("entropy_pool")?)))),
    },
    Provider {
        path: "core.validation.audit_log:GlyphChain",
        params: &["config", "event_bus"],
        construct: |kw| {
            wrap(GlyphChain::new(&*kw.get::<EnterpriseConfiguration>("config")?, kw.get("event_bus")?).map(Rc::new))
        },
    },
    Provider {
        path: "core.services:PiService",
        params: &["config", "event_bus"],
        construct: |kw| {
            wrap(PiService::new(&*kw.get::<EnterpriseConfiguration>("config")?, kw.get("event_bus")?).map(Rc::new))
        },
    },
    Provider {
        path: "core.services:FirmwareLoader",
        params: &["config"],
        construct: |kw| wrap(Ok(Rc::new(FirmwareLoader::new(&*kw.get::<EnterpriseConfiguration>("config")?)))),
    },
    Provider {
        path: "core.services:CpuService",
        params: &["config", "event_bus", "firmware_loader", "pi_service", "anticheat"],
        construct: |kw| {
            wrap(Ok(Rc::new(CpuService::new(
                kw.get("config")?,
                kw.get("event_bus")?,
                kw.get("firmware_loader")?,
                kw.get("pi_service")?,
                kw.get("anticheat")?,
            ))))
        },
    },
    Provider {
        path: "core.validation.policy:GlyphApprovalCommittee",
        params: &["config", "event_bus", "pi_service", "ledger"],
        construct: |kw| {
            wrap(
                GlyphApprovalCommittee::new(
                    &*kw.get::<EnterpriseConfiguration>("config")?,
                    kw.get("event_bus")?,
                    kw.get("pi_service")?,
                    kw.get("ledger")?,
                )
                .map(Rc::new),
            )
        },
    },
    Provider {
        path: "core.validation.normalization:LetterCaseNegotiator",
        params: &["config"],
        construct: |kw| wrap(LetterCaseNegotiator::new(&*kw.get::<EnterpriseConfiguration>("config")?).map(Rc::new)),
    },
    Provider {
        path: "core.output.terminal:TerminalRenderingEngine",
        params: &["config", "event_bus"],
        construct: |kw| {
            wrap(TerminalRenderingEngine::new(
                &*kw.get::<EnterpriseConfiguration>("config")?,
                &*kw.get::<EventBus>("event_bus")?,
            ))
        },
    },
    Provider {
        path: "core.security.drm:LicenseManager",
        params: &["config", "event_bus", "pi_service"],
        construct: |kw| {
            wrap(
                LicenseManager::new(
                    &*kw.get::<EnterpriseConfiguration>("config")?,
                    kw.get("event_bus")?,
                    kw.get("pi_service")?,
                )
                .map(Rc::new),
            )
        },
    },
    Provider {
        path: "core.security.anticheat:AntiCheatEngine",
        params: &["config", "event_bus"],
        construct: |kw| {
            wrap(
                AntiCheatEngine::new(&*kw.get::<EnterpriseConfiguration>("config")?, kw.get("event_bus")?).map(Rc::new),
            )
        },
    },
    Provider {
        path: "core.services:GreetingService",
        params: &["config", "event_bus", "cpu_service", "committee", "case_negotiator", "renderer", "drm", "anticheat"],
        construct: |kw| {
            wrap(Ok(Rc::new(GreetingService::new(
                kw.get("config")?,
                kw.get("event_bus")?,
                kw.get("cpu_service")?,
                kw.get("committee")?,
                kw.get("case_negotiator")?,
                kw.get("renderer")?,
                kw.get("drm")?,
                kw.get("anticheat")?,
            ))))
        },
    },
];

pub fn import_provider(path: &str) -> Result<&'static Provider> {
    if let Some(provider) = PROVIDERS.iter().find(|p| p.path == path) {
        return Ok(provider);
    }
    let (module, attr) = path.split_once(':').unwrap_or((path, ""));
    if PROVIDERS.iter().any(|p| p.path.split(':').next() == Some(module)) {
        Err(error::attribute_error(format!("module {} has no attribute {}", repr_str(module), repr_str(attr))))
    } else {
        Err(error::module_not_found_error(format!("No module named {}", repr_str(module))))
    }
}

#[derive(Default)]
pub struct Container {
    instances: HashMap<String, Instance>,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_instance(&mut self, name: &str, instance: Instance) {
        self.instances.insert(name.to_owned(), instance);
    }

    pub fn instantiate(&mut self, component: &Component) -> Result<Instance> {
        if self.instances.contains_key(&component.name) {
            return Err(error::container_error(format!("component already running: {}", component.name)));
        }
        let provider = import_provider(&component.provider)?;
        let mut values = Vec::with_capacity(component.requires.len());
        for (dep, _) in &component.requires {
            if !provider.params.contains(&dep.as_str()) {
                return Err(error::type_error(format!(
                    "{}.__init__() got an unexpected keyword argument {}",
                    provider.qualname(),
                    repr_str(dep)
                )));
            }
            values.push((dep.clone(), self.get_instance(dep)?));
        }
        if let Some(missing) = provider.params.iter().find(|p| !component.requires.iter().any(|(d, _)| d == *p)) {
            return Err(error::type_error(format!(
                "{}.__init__() missing 1 required positional argument: {}",
                provider.qualname(),
                repr_str(missing)
            )));
        }
        let instance = (provider.construct)(&Kwargs { values })?;
        self.instances.insert(component.name.clone(), instance.clone());
        crate::log_info!(
            LOG,
            "started {:<18} v{:<6} <- {}",
            component.name,
            component.version.to_string(),
            component.provider
        );
        Ok(instance)
    }

    pub fn get_instance(&self, name: &str) -> Result<Instance> {
        self.instances
            .get(name)
            .cloned()
            .ok_or_else(|| error::container_error(format!("component not running: {name}")))
    }

    pub fn get<T: Service>(&self, name: &str) -> Result<Rc<T>> {
        downcast(name, &self.get_instance(name)?)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.instances.contains_key(name)
    }
}
