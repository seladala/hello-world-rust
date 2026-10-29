// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Digital Rights Management.
//!
//! The greeting is licensed content. A signed license file grants entitlements;
//! the signing key is derived from the first 100 digits of pi, so the license
//! cannot even be checked until the pi oracles have reached consensus.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::events::EventBus;
use crate::core::json;
use crate::core::services::PiService;
use crate::core::values::{repr_str, Value};

const LOG: &str = "core.security.drm";

pub const KEY_DIGITS: usize = 100;

pub fn license_path() -> PathBuf {
    crate::core::package_dir().join("security").join("license.dat")
}

pub fn derive_key(pi_digits: &str) -> Vec<u8> {
    let prefix: String = pi_digits.chars().take(KEY_DIGITS).collect();
    Sha256::digest(format!("pi:{prefix}").as_bytes()).to_vec()
}

pub fn canonical(body: &Value) -> Vec<u8> {
    json::dumps(body, true, &json::COMPACT_SEPARATORS).into_bytes()
}

pub fn sign(body: &Value, key: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(&canonical(body));
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn compare_digest(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut difference = a.len() ^ b.len();
    for (index, &byte) in a.iter().enumerate() {
        difference |= (byte ^ b.get(index).copied().unwrap_or(!byte)) as usize;
    }
    difference == 0
}

/// Today's date (UTC) as (year, month, day).
fn today() -> (i64, u32, u32) {
    let secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    civil_from_days(secs.div_euclid(86_400))
}

/// Days since 1970-01-01 to a proleptic Gregorian calendar date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn from_iso_format(text: &str) -> Result<(i64, u32, u32)> {
    let invalid = || error::value_error(format!("Invalid isoformat string: {}", repr_str(text)));
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(invalid());
    }
    let number = |range: std::ops::Range<usize>| -> Result<i64> {
        let part = &text[range];
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(invalid());
        }
        part.parse().map_err(|_| invalid())
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if year < 1 || !(1..=12).contains(&month) || day < 1 || day > days_in_month[(month - 1) as usize] {
        return Err(error::value_error(if (1..=12).contains(&month) {
            "day is out of range for month"
        } else {
            "month must be in 1..12"
        }));
    }
    Ok((year, month as u32, day as u32))
}

pub struct LicenseManager {
    bus: Rc<EventBus>,
    pi: Rc<PiService>,
    product: String,
    entitlements: RefCell<BTreeSet<String>>,
}

impl LicenseManager {
    pub fn new(config: &EnterpriseConfiguration, event_bus: Rc<EventBus>, pi_service: Rc<PiService>) -> Result<Self> {
        Ok(LicenseManager {
            bus: event_bus,
            pi: pi_service,
            product: config.get("security", "product_id")?,
            entitlements: RefCell::new(BTreeSet::new()),
        })
    }

    pub fn require(&self, entitlement: &str) -> Result<()> {
        if !self.entitlements.borrow().contains(entitlement) {
            return Err(error::license_error(format!("license does not grant {}", repr_str(entitlement))));
        }
        Ok(())
    }
}

impl Service for LicenseManager {
    fn on_start(&self) -> Result<()> {
        let mut document = std::fs::read(license_path())
            .map_err(|e| e.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
            .and_then(|text| json::loads(&text))
            .map_err(|e| error::license_error(format!("license file unreadable: {e}")))?;
        if !matches!(document, Value::Dict(_)) {
            return Err(error::attribute_error(format!(
                "{} object has no attribute 'pop'",
                repr_str(document.type_name())
            )));
        }
        let signature = match document.pop("signature").unwrap_or(Value::str("")) {
            Value::Str(signature) => signature,
            other => {
                return Err(error::type_error(format!(
                    "unsupported operand types(s) or combination of types: {} and 'str'",
                    repr_str(other.type_name())
                )))
            }
        };
        let expected = sign(&document, &derive_key(&self.pi.digits()?));
        if !compare_digest(&signature, &expected) {
            return Err(error::license_error("license signature invalid"));
        }
        let product = document.get("product").cloned().unwrap_or(Value::None);
        if product != Value::str(&self.product) {
            return Err(error::license_error(format!(
                "license is for {}, not {}",
                product.repr(),
                repr_str(&self.product)
            )));
        }
        match document.get("expires") {
            None | Some(Value::None) => {}
            Some(Value::Str(expires)) => {
                if from_iso_format(expires)? < today() {
                    return Err(error::license_error(format!("license expired on {expires}")));
                }
            }
            Some(other) => {
                return Err(error::type_error(format!(
                    "fromisoformat: argument must be str, not {}",
                    other.type_name()
                )))
            }
        }
        let entitlements: BTreeSet<String> = match document.get("entitlements") {
            None => BTreeSet::new(),
            Some(Value::List(items)) | Some(Value::Tuple(items)) => items.iter().map(Value::py_str).collect(),
            Some(Value::Str(text)) => text.chars().map(String::from).collect(),
            Some(Value::Dict(items)) => items.iter().map(|(k, _)| k.clone()).collect(),
            Some(other) => {
                return Err(error::type_error(format!("{} object is not iterable", repr_str(other.type_name()))))
            }
        };
        let count = entitlements.len();
        *self.entitlements.borrow_mut() = entitlements;
        let field = |key: &str| document.get(key).cloned().ok_or_else(|| error::key_error(repr_str(key)));
        let licensee = field("licensee")?;
        self.bus.publish("drm.license.activated", licensee.clone())?;
        crate::log_info!(
            LOG,
            "drm: {} edition licensed to {} ({count} entitlements)",
            field("edition")?.py_str(),
            licensee.py_str()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_755), (2026, 10, 29));
        assert!(from_iso_format("2026-02-29").is_err());
        assert_eq!(from_iso_format("2024-02-29").unwrap(), (2024, 2, 29));
    }

    #[test]
    fn constant_time_comparison() {
        assert!(compare_digest("abc", "abc"));
        assert!(!compare_digest("abc", "abd"));
        assert!(!compare_digest("abc", "ab"));
    }
}
