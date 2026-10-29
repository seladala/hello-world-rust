//! Letter case is decided through structured multi-round negotiation.

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::values::repr_str;

const LOG: &str = "core.validation.normalization";

fn is_cased(c: char) -> bool {
    c.is_uppercase() || c.is_lowercase()
}

pub fn upper(text: &str) -> String {
    text.to_uppercase()
}

pub fn lower(text: &str) -> String {
    text.to_lowercase()
}

/// Uppercase the first letter of every run of cased characters, lowercase the rest.
pub fn title(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut previous_cased = false;
    for c in text.chars() {
        if previous_cased {
            out.extend(c.to_lowercase());
        } else {
            out.extend(c.to_uppercase());
        }
        previous_cased = is_cased(c);
    }
    out
}

/// Uppercase the first character, lowercase everything else.
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
        None => String::new(),
    }
}

pub type CaseTransform = fn(&str) -> String;

pub const PROPOSALS: [(&str, CaseTransform); 4] =
    [("UPPER", upper), ("Title", title), ("Sentence", capitalize), ("lower", lower)];

pub struct LetterCaseNegotiator {
    mandate: String,
    separator_policy: String,
}

impl Service for LetterCaseNegotiator {}

impl LetterCaseNegotiator {
    pub fn new(config: &EnterpriseConfiguration) -> Result<Self> {
        Ok(LetterCaseNegotiator {
            mandate: config.get("greeting", "case")?,
            separator_policy: config.get("greeting", "separator_policy")?,
        })
    }

    pub fn negotiate(&self, text: &str) -> Result<String> {
        for (round_no, (name, transform)) in PROPOSALS.iter().enumerate() {
            let proposal = transform(text);
            let accepted = name.to_lowercase() == self.mandate.to_lowercase();
            crate::log_info!(
                LOG,
                "case negotiation round {}: {:<8} -> {} {}",
                round_no + 1,
                name,
                repr_str(&proposal),
                if accepted { "ACCEPTED" } else { "rejected, escalating" }
            );
            if accepted {
                return Ok(self.enforce_separators(&proposal));
            }
        }
        Err(error::negotiation_deadlock(format!("no proposal satisfies mandate {}", repr_str(&self.mandate))))
    }

    pub(crate) fn enforce_separators(&self, text: &str) -> String {
        if self.separator_policy == "single-space" {
            return text.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        text.to_owned()
    }
}
