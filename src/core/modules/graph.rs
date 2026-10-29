//! Dependency resolution for internal components.
//!
//! Reads a JSON manifest, validates every version constraint, detects cycles
//! and produces a deterministic topological boot order (Kahn's algorithm).

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::Path;

use super::versioning::{satisfies, Version};
use crate::core::error::{self, Result};
use crate::core::json;
use crate::core::values::{repr_str, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub version: Version,
    pub provider: String,
    pub requires: Vec<(String, String)>,
    pub description: String,
}

impl Component {
    pub fn new(name: &str, version: Version, provider: &str, requires: &[(&str, &str)]) -> Component {
        Component {
            name: name.to_owned(),
            version,
            provider: provider.to_owned(),
            requires: requires.iter().map(|(d, s)| (d.to_string(), s.to_string())).collect(),
            description: String::new(),
        }
    }
}

/// Components in declaration order, keyed by name.
pub type Components = Vec<Component>;

fn field<'a>(entry: &'a Value, key: &str) -> Result<&'a Value> {
    entry.get(key).ok_or_else(|| error::key_error(repr_str(key)))
}

fn string_field(entry: &Value, key: &str) -> Result<String> {
    field(entry, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| error::type_error(format!("manifest field {} must be a string", repr_str(key))))
}

pub fn load_manifest(path: &Path) -> Result<Components> {
    let text = std::fs::read_to_string(path).map_err(|e| error::os_error(format!("{e}: {}", path.display())))?;
    let data = json::loads(&text).map_err(error::value_error)?;
    let Value::List(entries) = field(&data, "components")? else {
        return Err(error::type_error("manifest 'components' must be a list"));
    };
    let mut components: Components = Vec::new();
    for entry in entries {
        let requires = match entry.get("requires") {
            None => Vec::new(),
            Some(Value::Dict(items)) => items
                .iter()
                .map(|(dep, spec)| {
                    spec.as_str()
                        .map(|s| (dep.clone(), s.to_owned()))
                        .ok_or_else(|| error::type_error("version constraints must be strings"))
                })
                .collect::<Result<_>>()?,
            Some(_) => return Err(error::type_error("manifest 'requires' must be an object")),
        };
        let component = Component {
            name: string_field(entry, "name")?,
            version: Version::parse(&string_field(entry, "version")?)?,
            provider: string_field(entry, "provider")?,
            requires,
            description: entry.get("description").and_then(Value::as_str).unwrap_or("").to_owned(),
        };
        if components.iter().any(|c| c.name == component.name) {
            return Err(error::dependency_error(format!("component declared twice: {}", component.name)));
        }
        components.push(component);
    }
    Ok(components)
}

pub fn resolve(components: &[Component]) -> Result<Vec<Component>> {
    let index_of = |name: &str| components.iter().position(|c| c.name == name);
    let mut indegree = vec![0usize; components.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); components.len()];
    for (index, component) in components.iter().enumerate() {
        for (dep, spec) in &component.requires {
            let Some(dep_index) = index_of(dep) else {
                return Err(error::dependency_error(format!(
                    "{} requires missing component {}",
                    component.name,
                    repr_str(dep)
                )));
            };
            if !satisfies(&components[dep_index].version, spec)? {
                return Err(error::dependency_error(format!(
                    "{} requires {dep} {spec}, found {}",
                    component.name, components[dep_index].version
                )));
            }
            indegree[index] += 1;
            dependents[dep_index].push(index);
        }
    }

    let mut ready: BinaryHeap<Reverse<(&str, usize)>> = components
        .iter()
        .enumerate()
        .filter(|(i, _)| indegree[*i] == 0)
        .map(|(i, c)| Reverse((c.name.as_str(), i)))
        .collect();
    let mut order = Vec::with_capacity(components.len());
    while let Some(Reverse((_, index))) = ready.pop() {
        order.push(components[index].clone());
        for &dependent in &dependents[index] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                ready.push(Reverse((components[dependent].name.as_str(), dependent)));
            }
        }
    }
    if order.len() != components.len() {
        let mut stuck: Vec<&str> =
            components.iter().enumerate().filter(|(i, _)| indegree[*i] > 0).map(|(_, c)| c.name.as_str()).collect();
        stuck.sort_unstable();
        return Err(error::dependency_error(format!("dependency cycle among: {}", stuck.join(", "))));
    }
    Ok(order)
}
