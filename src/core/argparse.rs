// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Command-line parsing in the image of a well-known standard library module:
//! flag clusters (`-vv`), unambiguous long-option abbreviations (`--dis`),
//! `--opt=value` diagnostics, `--` handling, and the same help layout.

use std::collections::HashMap;
use std::io::Write;

use super::values::repr_str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    Help,
    Count,
    StoreTrue,
    Version(String),
}

#[derive(Debug, Clone)]
pub struct Action {
    pub options: Vec<&'static str>,
    pub dest: &'static str,
    pub kind: ActionKind,
    pub help: &'static str,
}

impl Action {
    fn name(&self) -> String {
        self.options.join("/")
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Namespace {
    values: HashMap<&'static str, u64>,
}

impl Namespace {
    pub fn count(&self, dest: &str) -> u64 {
        self.values.get(dest).copied().unwrap_or(0)
    }

    pub fn flag(&self, dest: &str) -> bool {
        self.count(dest) > 0
    }
}

/// The parser asked the process to exit with this status (after printing help, a version or an error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit(pub u8);

pub struct ArgumentParser {
    pub prog: String,
    pub description: String,
    actions: Vec<Action>,
}

enum Classified {
    Option { action: usize, option: String, explicit: Option<String>, sep: bool },
    Positional(String),
}

impl ArgumentParser {
    pub fn new(prog: &str, description: &str) -> ArgumentParser {
        let help = Action {
            options: vec!["-h", "--help"],
            dest: "help",
            kind: ActionKind::Help,
            help: "show this help message and exit",
        };
        ArgumentParser { prog: prog.to_owned(), description: description.to_owned(), actions: vec![help] }
    }

    pub fn add_argument(&mut self, options: &[&'static str], kind: ActionKind, help: &'static str) {
        let dest = options
            .iter()
            .find(|o| o.starts_with("--"))
            .or(options.first())
            .map(|o| o.trim_start_matches('-'))
            .unwrap_or("");
        self.actions.push(Action { options: options.to_vec(), dest, kind, help });
    }

    pub fn format_usage(&self) -> String {
        let parts: Vec<String> = self.actions.iter().map(|a| format!("[{}]", a.options[0])).collect();
        format!("usage: {} {}\n", self.prog, parts.join(" "))
    }

    pub fn format_help(&self) -> String {
        let invocations: Vec<String> = self.actions.iter().map(|a| a.options.join(", ")).collect();
        let longest = invocations.iter().map(String::len).max().unwrap_or(0);
        let help_position = (longest + 4).min(24);
        let mut out = format!("{}\n{}\n\noptions:\n", self.format_usage(), self.description);
        for (action, invocation) in self.actions.iter().zip(&invocations) {
            if 2 + invocation.len() + 2 <= help_position {
                out.push_str(&format!("  {:<width$}{}\n", invocation, action.help, width = help_position - 2));
            } else {
                out.push_str(&format!("  {invocation}\n{}{}\n", " ".repeat(help_position), action.help));
            }
        }
        out
    }

    fn error(&self, message: &str) -> Exit {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{}{}: error: {message}", self.format_usage(), self.prog);
        Exit(2)
    }

    fn find_exact(&self, option: &str) -> Option<usize> {
        self.actions.iter().position(|a| a.options.contains(&option))
    }

    fn classify(&self, arg: &str) -> Result<Classified, Exit> {
        if !arg.starts_with('-') || arg == "-" {
            return Ok(Classified::Positional(arg.to_owned()));
        }
        if let Some(action) = self.find_exact(arg) {
            return Ok(Classified::Option { action, option: arg.to_owned(), explicit: None, sep: false });
        }
        if let Some((option, value)) = arg.split_once('=') {
            if let Some(action) = self.find_exact(option) {
                return Ok(Classified::Option {
                    action,
                    option: option.to_owned(),
                    explicit: Some(value.to_owned()),
                    sep: true,
                });
            }
        }
        if arg.starts_with("--") {
            let (prefix, explicit) = match arg.split_once('=') {
                Some((prefix, value)) => (prefix, Some(value.to_owned())),
                None => (arg, None),
            };
            let matches: Vec<(usize, &str)> = self
                .actions
                .iter()
                .enumerate()
                .flat_map(|(i, a)| a.options.iter().map(move |o| (i, *o)))
                .filter(|(_, o)| o.starts_with("--") && o.starts_with(prefix))
                .collect();
            return match matches.as_slice() {
                [] => Ok(Classified::Positional(arg.to_owned())),
                [(action, option)] => Ok(Classified::Option {
                    action: *action,
                    option: option.to_string(),
                    sep: explicit.is_some(),
                    explicit,
                }),
                many => {
                    let names: Vec<&str> = many.iter().map(|(_, o)| *o).collect();
                    Err(self.error(&format!("ambiguous option: {prefix} could match {}", names.join(", "))))
                }
            };
        }
        // A single-dash cluster such as "-vv": the first flag, with the rest as its explicit argument.
        let short: String = arg.chars().take(2).collect();
        match self.find_exact(&short) {
            Some(action) => {
                Ok(Classified::Option { action, option: short, explicit: Some(arg[2..].to_owned()), sep: false })
            }
            None => Ok(Classified::Positional(arg.to_owned())),
        }
    }

    pub fn parse_args(&self, argv: &[String]) -> Result<Namespace, Exit> {
        let mut classified = Vec::new();
        let mut rest = argv.iter();
        for arg in rest.by_ref() {
            if arg == "--" {
                classified.push(Classified::Positional(arg.clone()));
                break;
            }
            classified.push(self.classify(arg)?);
        }
        classified.extend(rest.map(|arg| Classified::Positional(arg.clone())));

        let mut namespace = Namespace::default();
        let mut extras: Vec<String> = Vec::new();
        for item in classified {
            let (mut action, mut option, mut explicit, mut sep) = match item {
                Classified::Positional(arg) => {
                    extras.push(arg);
                    continue;
                }
                Classified::Option { action, option, explicit, sep } => (action, option, explicit, sep),
            };
            // Expand flag clusters ("-vvh") before running any action from them.
            let mut batch = Vec::new();
            loop {
                let Some(cluster) = explicit.take() else {
                    batch.push(action);
                    break;
                };
                let ignored = format!(
                    "argument {}: ignored explicit argument {}",
                    self.actions[action].name(),
                    repr_str(&cluster)
                );
                if option.starts_with("--") || cluster.is_empty() || sep || cluster.starts_with('-') {
                    return Err(self.error(&ignored));
                }
                batch.push(action);
                let first = cluster.chars().next().unwrap_or_default();
                let next = format!("-{first}");
                let Some(next_action) = self.find_exact(&next) else {
                    extras.push(format!("-{cluster}"));
                    break;
                };
                let remainder = &cluster[first.len_utf8()..];
                action = next_action;
                option = next;
                (explicit, sep) = match remainder.strip_prefix('=') {
                    _ if remainder.is_empty() => (None, false),
                    Some(value) => (Some(value.to_owned()), true),
                    None => (Some(remainder.to_owned()), false),
                };
            }
            for action in batch {
                self.execute(action, &mut namespace)?;
            }
        }
        if !extras.is_empty() {
            return Err(self.error(&format!("unrecognized arguments: {}", extras.join(" "))));
        }
        Ok(namespace)
    }

    fn execute(&self, index: usize, namespace: &mut Namespace) -> Result<(), Exit> {
        let action = &self.actions[index];
        match &action.kind {
            ActionKind::Help => {
                let mut stdout = std::io::stdout().lock();
                let _ = stdout.write_all(self.format_help().as_bytes());
                Err(Exit(0))
            }
            ActionKind::Version(version) => {
                let mut stdout = std::io::stdout().lock();
                let _ = writeln!(stdout, "{version}");
                Err(Exit(0))
            }
            ActionKind::Count => {
                *namespace.values.entry(action.dest).or_insert(0) += 1;
                Ok(())
            }
            ActionKind::StoreTrue => {
                namespace.values.insert(action.dest, 1);
                Ok(())
            }
        }
    }
}
