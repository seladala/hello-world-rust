// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! The Glyph Approval Committee.
//!
//! No character reaches the terminal without being reviewed by five independent
//! officers, majority-voted, stamped, and recorded on the GlyphChain.

use std::rc::Rc;

use super::audit_log::GlyphChain;
use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::events::EventBus;
use crate::core::services::PiService;
use crate::core::values::{is_printable, repr_str, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub officer: &'static str,
    pub approved: bool,
    pub remark: String,
    pub veto: bool,
}

pub trait Officer {
    fn title(&self) -> &'static str;

    fn has_veto(&self) -> bool {
        false
    }

    fn review(&self, glyph: char, position: usize, pi_digits: &str) -> Verdict;

    fn verdict(&self, approved: bool, remark: String) -> Verdict {
        Verdict { officer: self.title(), approved, remark, veto: self.has_veto() && !approved }
    }
}

pub struct PrintabilityAuditor;

impl Officer for PrintabilityAuditor {
    fn title(&self) -> &'static str {
        "Printability Auditor"
    }

    fn review(&self, glyph: char, _position: usize, _pi_digits: &str) -> Verdict {
        self.verdict(is_printable(glyph), "visible to the naked eye".to_owned())
    }
}

pub struct AsciiPurist;

impl Officer for AsciiPurist {
    fn title(&self) -> &'static str {
        "ASCII Purist"
    }

    fn review(&self, glyph: char, _position: usize, _pi_digits: &str) -> Verdict {
        self.verdict((glyph as u32) < 128, "no Unicode nonsense detected".to_owned())
    }
}

pub struct PiNumerologist;

impl Officer for PiNumerologist {
    fn title(&self) -> &'static str {
        "Pi Numerologist"
    }

    fn review(&self, glyph: char, position: usize, pi_digits: &str) -> Verdict {
        let digits = pi_digits.as_bytes();
        let omen = (digits[position % digits.len()] - b'0') as u64;
        self.verdict((glyph as u64 * omen).is_multiple_of(1), format!("the digit {omen} looks favourable"))
    }
}

pub struct VowelEqualityOfficer;

impl Officer for VowelEqualityOfficer {
    fn title(&self) -> &'static str {
        "Vowel Equality Officer"
    }

    fn review(&self, glyph: char, _position: usize, _pi_digits: &str) -> Verdict {
        let kind = if "aeiou".contains(glyph) {
            "vowel"
        } else if glyph.is_alphabetic() {
            "consonant"
        } else {
            "neutral entity"
        };
        self.verdict(true, format!("{kind} treated with equal respect"))
    }
}

pub struct ChiefHelloOfficer;

impl ChiefHelloOfficer {
    const ALLOWED: &'static str = "helowrd ";
}

impl Officer for ChiefHelloOfficer {
    fn title(&self) -> &'static str {
        "Chief Hello Officer"
    }

    fn has_veto(&self) -> bool {
        true
    }

    fn review(&self, glyph: char, _position: usize, _pi_digits: &str) -> Verdict {
        let lowered: Vec<char> = glyph.to_lowercase().collect();
        let on_brand = matches!(lowered[..], [c] if Self::ALLOWED.contains(c));
        self.verdict(on_brand, "on-brand".to_owned())
    }
}

pub fn all_officers() -> Vec<Box<dyn Officer>> {
    vec![
        Box::new(PrintabilityAuditor),
        Box::new(AsciiPurist),
        Box::new(PiNumerologist),
        Box::new(VowelEqualityOfficer),
        Box::new(ChiefHelloOfficer),
    ]
}

pub struct GlyphApprovalCommittee {
    officers: Vec<Box<dyn Officer>>,
    majority: f64,
    bus: Rc<EventBus>,
    pi: Rc<PiService>,
    ledger: Rc<GlyphChain>,
}

impl Service for GlyphApprovalCommittee {}

impl GlyphApprovalCommittee {
    pub fn new(
        config: &EnterpriseConfiguration,
        event_bus: Rc<EventBus>,
        pi_service: Rc<PiService>,
        ledger: Rc<GlyphChain>,
    ) -> Result<Self> {
        let members = config.get_int("committee", "members")?.max(0) as usize;
        Ok(GlyphApprovalCommittee {
            officers: all_officers().into_iter().take(members).collect(),
            majority: config.get_float("committee", "required_majority")?,
            bus: event_bus,
            pi: pi_service,
            ledger,
        })
    }

    pub fn review(&self, text: &str) -> Result<String> {
        let digits = self.pi.digits()?;
        let mut approved = String::new();
        for (position, glyph) in text.chars().enumerate() {
            let verdicts: Vec<Verdict> =
                self.officers.iter().map(|officer| officer.review(glyph, position, &digits)).collect();
            if verdicts.is_empty() {
                return Err(error::runtime_error("division by zero"));
            }
            let ayes = verdicts.iter().filter(|v| v.approved).count();
            let vetoed: Vec<Value> = verdicts.iter().filter(|v| v.veto).map(|v| Value::str(v.officer)).collect();
            if !vetoed.is_empty() || (ayes as f64 / verdicts.len() as f64) < self.majority {
                return Err(error::glyph_rejected(format!(
                    "glyph {} at {position} rejected (ayes={ayes}, vetoes={})",
                    repr_str(&glyph.to_string()),
                    Value::List(vetoed).repr()
                )));
            }
            self.ledger.append(Value::dict([
                ("position", Value::Int(position as i64)),
                ("glyph", Value::str(glyph.to_string())),
                ("ayes", Value::Int(ayes as i64)),
                ("stamps", Value::List(verdicts.iter().map(|v| Value::str(v.officer)).collect())),
            ]))?;
            self.bus.publish(
                "glyph.approved",
                Value::dict([
                    ("glyph", Value::str(glyph.to_string())),
                    ("ayes", Value::str(format!("{ayes}/{}", verdicts.len()))),
                ]),
            )?;
            approved.push(glyph);
        }
        self.ledger.verify()?;
        Ok(approved)
    }
}
