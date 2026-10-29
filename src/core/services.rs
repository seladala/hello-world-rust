//! Core services. Each one does a small job in the largest possible number of steps.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use super::config::EnterpriseConfiguration;
use super::container::Service;
use super::diagnostics::arithmetic::{peano_length, to_int};
use super::error::{self, Result};
use super::events::EventBus;
use super::numerics::PiConsensusProtocol;
use super::output::terminal::TerminalRenderingEngine;
use super::runtime::{assemble, disassemble, AssembledProgram, Machine};
use super::security::anticheat::AntiCheatEngine;
use super::security::drm::LicenseManager;
use super::validation::normalization::LetterCaseNegotiator;
use super::validation::policy::GlyphApprovalCommittee;
use super::values::Value;

const LOG: &str = "core.services";

pub fn programs_dir() -> PathBuf {
    super::package_dir().join("runtime").join("firmware")
}

pub const PORT_TTY: u8 = 0x01;
pub const PORT_STATUS: u8 = 0x02;
pub const PORT_FIB: u8 = 0x10;
pub const PORT_RAM: u8 = 0x11;
pub const STATUS_GOOD: u16 = 0x600D;

fn watchdog(config: &EnterpriseConfiguration) -> Result<u64> {
    Ok(config.get_int("cpu", "watchdog_cycles")?.max(0) as u64)
}

pub struct PiService {
    protocol: PiConsensusProtocol,
    count: usize,
    bus: Rc<EventBus>,
    digits: RefCell<Option<String>>,
}

impl Service for PiService {}

impl PiService {
    pub fn new(config: &EnterpriseConfiguration, event_bus: Rc<EventBus>) -> Result<Self> {
        Ok(PiService {
            protocol: PiConsensusProtocol::new(config.get_int("pi", "quorum")?.max(0) as usize),
            count: config.get_int("pi", "digits")?.max(0) as usize,
            bus: event_bus,
            digits: RefCell::new(None),
        })
    }

    pub fn digits(&self) -> Result<String> {
        if let Some(digits) = self.digits.borrow().as_ref() {
            return Ok(digits.clone());
        }
        let digits = self.protocol.agree(self.count)?;
        *self.digits.borrow_mut() = Some(digits.clone());
        let head: String = digits.chars().take(10).collect();
        self.bus.publish("pi.agreed", Value::str(format!("{head}...")))?;
        Ok(digits)
    }
}

pub struct FirmwareLoader {
    cache: RefCell<HashMap<String, Rc<AssembledProgram>>>,
}

impl Service for FirmwareLoader {}

impl FirmwareLoader {
    pub fn new(_config: &EnterpriseConfiguration) -> Self {
        FirmwareLoader { cache: RefCell::new(HashMap::new()) }
    }

    pub fn load(&self, name: &str) -> Result<Rc<AssembledProgram>> {
        if let Some(program) = self.cache.borrow().get(name) {
            return Ok(program.clone());
        }
        let path = programs_dir().join(name);
        let source = std::fs::read_to_string(&path)
            .map_err(|e| error::file_not_found_error(format!("{e}: {}", path.display())))?;
        let program = Rc::new(assemble(&source)?);
        let listing = disassemble(&program.image);
        crate::log_info!(
            LOG,
            "firmware {name}: {} bytes, {} disassembled lines, {} symbols",
            program.image.len(),
            listing.len(),
            program.symbols.len()
        );
        for line in &listing {
            crate::log_debug!(LOG, "  {line}");
        }
        self.cache.borrow_mut().insert(name.to_owned(), program.clone());
        Ok(program)
    }
}

pub struct CpuService {
    config: Rc<EnterpriseConfiguration>,
    bus: Rc<EventBus>,
    loader: Rc<FirmwareLoader>,
    pi: Rc<PiService>,
    anticheat: Rc<AntiCheatEngine>,
}

impl CpuService {
    pub fn new(
        config: Rc<EnterpriseConfiguration>,
        event_bus: Rc<EventBus>,
        firmware_loader: Rc<FirmwareLoader>,
        pi_service: Rc<PiService>,
        anticheat: Rc<AntiCheatEngine>,
    ) -> Self {
        CpuService { config, bus: event_bus, loader: firmware_loader, pi: pi_service, anticheat }
    }

    pub fn run_self_test(&self) -> Result<()> {
        let mut machine = Machine::new();
        machine.load(&self.loader.load(&self.config.get("cpu", "self_test")?)?.image, 0)?;
        let results: Rc<RefCell<Vec<(String, i64)>>> = Rc::default();
        let record = |key: &'static str| {
            let results = results.clone();
            move |value: u16| {
                let mut results = results.borrow_mut();
                match results.iter_mut().find(|(k, _)| k == key) {
                    Some(slot) => slot.1 = value as i64,
                    None => results.push((key.to_owned(), value as i64)),
                }
            }
        };
        machine.attach_output(PORT_FIB, record("fib"));
        machine.attach_output(PORT_RAM, record("ram_errors"));
        let cycles = machine.run(watchdog(&self.config)?)?;
        let results = results.borrow().clone();
        let mut expected = vec![("fib".to_owned(), 6765), ("ram_errors".to_owned(), 0)];
        let mut actual = results.clone();
        expected.sort();
        actual.sort();
        let as_dict = |items: &[(String, i64)]| Value::dict(items.iter().map(|(k, v)| (k.clone(), Value::Int(*v))));
        if actual != expected {
            return Err(error::machine_fault(format!("POST failed: {}", as_dict(&results).repr())));
        }
        let mut payload = vec![("cycles".to_owned(), Value::Int(cycles as i64))];
        payload.extend(results.iter().map(|(k, v)| (k.clone(), Value::Int(*v))));
        self.bus.publish("cpu.post.passed", Value::Dict(payload))?;
        Ok(())
    }

    pub fn execute_firmware(&self) -> Result<String> {
        let firmware = self.loader.load(&self.config.get("cpu", "firmware")?)?;
        let mut machine = Machine::new();
        machine.load(&firmware.image, 0)?;
        let digits: Vec<u8> = self.pi.digits()?.bytes().map(|d| d - b'0').collect();
        let address = self.config.get_int("cpu", "pi_digits_address")?;
        let address = usize::try_from(address).map_err(|_| error::value_error("negative pi_digits_address"))?;
        machine.load(&digits, address)?;
        let output: Rc<RefCell<String>> = Rc::default();
        let status: Rc<RefCell<Vec<u16>>> = Rc::default();
        let sink = output.clone();
        machine.attach_output(PORT_TTY, move |v| sink.borrow_mut().push(char::from((v & 0xFF) as u8)));
        let sink = status.clone();
        machine.attach_output(PORT_STATUS, move |v| sink.borrow_mut().push(v));
        let cycles = machine.run(watchdog(&self.config)?)?;
        self.anticheat.verify_code_segment(&machine, &firmware.image)?;
        let status = status.borrow().clone();
        if status != [STATUS_GOOD] {
            let reported = Value::List(status.iter().map(|&s| Value::Int(s as i64)).collect());
            return Err(error::machine_fault(format!("firmware reported bad status: {}", reported.repr())));
        }
        let clock_hz = self.config.get_int("cpu", "clock_hz")?;
        if clock_hz == 0 {
            return Err(error::runtime_error("division by zero"));
        }
        let emulated_seconds = cycles as f64 / clock_hz as f64;
        self.bus.publish(
            "cpu.firmware.halted",
            Value::dict([("cycles", Value::Int(cycles as i64)), ("emulated_seconds", Value::Float(emulated_seconds))]),
        )?;
        let text = output.borrow().clone();
        Ok(text)
    }
}

impl Service for CpuService {
    fn on_start(&self) -> Result<()> {
        self.run_self_test()
    }
}

pub struct GreetingService {
    config: Rc<EnterpriseConfiguration>,
    bus: Rc<EventBus>,
    cpu: Rc<CpuService>,
    committee: Rc<GlyphApprovalCommittee>,
    negotiator: Rc<LetterCaseNegotiator>,
    #[allow(dead_code)]
    renderer: Rc<TerminalRenderingEngine>,
    drm: Rc<LicenseManager>,
    anticheat: Rc<AntiCheatEngine>,
}

impl Service for GreetingService {}

impl GreetingService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Rc<EnterpriseConfiguration>,
        event_bus: Rc<EventBus>,
        cpu_service: Rc<CpuService>,
        committee: Rc<GlyphApprovalCommittee>,
        case_negotiator: Rc<LetterCaseNegotiator>,
        renderer: Rc<TerminalRenderingEngine>,
        drm: Rc<LicenseManager>,
        anticheat: Rc<AntiCheatEngine>,
    ) -> Self {
        GreetingService {
            config,
            bus: event_bus,
            cpu: cpu_service,
            committee,
            negotiator: case_negotiator,
            renderer,
            drm,
            anticheat,
        }
    }

    pub fn produce(&self) -> Result<String> {
        self.drm.require("greeting.produce")?;
        let raw = self.cpu.execute_firmware()?;
        for (index, glyph) in raw.chars().enumerate() {
            self.bus.publish(
                "glyph.received",
                Value::Tuple(vec![Value::Int(index as i64), Value::str(glyph.to_string())]),
            )?;
        }
        let approved = self.committee.review(&raw)?;
        let negotiated = self.negotiator.negotiate(&approved)?;
        if to_int(&peano_length(negotiated.chars())) as i64 != self.config.get_int("greeting", "expected_length")? {
            return Err(error::value_error("greeting failed Peano length verification"));
        }
        Ok(negotiated)
    }

    pub fn deliver(&self, message: &str) -> Result<()> {
        self.drm.require("greeting.render")?;
        self.anticheat.scan()?;
        for glyph in message.chars() {
            self.bus.publish("glyph.render", Value::str(glyph.to_string()))?;
        }
        self.bus.publish("transmission.complete", Value::Int(message.chars().count() as i64))?;
        Ok(())
    }
}
