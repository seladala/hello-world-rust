use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use hello_world::core::diagnostics::arithmetic::{peano_length, to_int};
use hello_world::core::modules::{resolve, satisfies, Component, Version};
use hello_world::core::numerics::{high_precision, series, streaming};
use hello_world::core::runtime::{assemble, Machine};
use hello_world::core::security::integrity::{checksums_path, compute_manifest, format_manifest};
use hello_world::core::ROOT_ENV;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const BINARY: &str = env!("CARGO_BIN_EXE_hello-world");

fn run(command: &mut Command) -> Output {
    command.output().expect("failed to run hello-world")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

mod end_to_end {
    use super::*;

    #[test]
    fn prints_hello_world() {
        let result = run(Command::new(BINARY).env_remove(ROOT_ENV));
        assert!(result.status.success(), "{}", stderr(&result));
        assert_eq!(stdout(&result), "hello world\n");
    }

    #[test]
    fn main_is_exactly_67000_lines() {
        let text = std::fs::read_to_string(Path::new(ROOT).join("src").join("main.rs")).unwrap();
        assert_eq!(text.lines().count(), 67_000);
    }
}

mod security {
    use super::*;

    /// A throwaway copy of the protected tree that the binary is pointed at.
    struct Sandbox(PathBuf);

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let target = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_tree(&path, &target);
            } else {
                std::fs::copy(&path, &target).unwrap();
            }
        }
    }

    fn run_tampered(relative_path: &str, old: &str, new: &str, refresh_checksums: bool) -> Output {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = format!("hello-world-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed));
        let sandbox = Sandbox(std::env::temp_dir().join(unique));
        let src = Path::new(ROOT).join("src");
        std::fs::create_dir_all(sandbox.0.join("src")).unwrap();
        for file in ["main.rs", "lib.rs"] {
            std::fs::copy(src.join(file), sandbox.0.join("src").join(file)).unwrap();
        }
        copy_tree(&src.join("core"), &sandbox.0.join("src").join("core"));

        let target = sandbox.0.join(relative_path);
        let text = std::fs::read_to_string(&target).unwrap();
        assert!(text.contains(old), "{old:?} not found in {relative_path}");
        std::fs::write(&target, text.replace(old, new)).unwrap();
        if refresh_checksums {
            let manifest = compute_manifest(&sandbox.0).unwrap();
            std::fs::write(checksums_path(&sandbox.0), format_manifest(&manifest)).unwrap();
        }
        run(Command::new(BINARY).env(ROOT_ENV, &sandbox.0))
    }

    #[test]
    fn modified_file_is_rejected() {
        let result = run_tampered("src/core/settings.ini", "case = lower", "case = UPPER", false);
        assert_eq!(result.status.code(), Some(3));
        assert!(stderr(&result).contains("modified: src/core/settings.ini"), "{}", stderr(&result));
    }

    #[test]
    fn forged_license_is_rejected() {
        let result = run_tampered("src/core/security/license.dat", "\"Everyone\"", "\"Pirate\"", true);
        assert_eq!(result.status.code(), Some(3));
        assert!(stderr(&result).contains("license signature invalid"), "{}", stderr(&result));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn debugger_is_detected() {
        let probe = Command::new("strace").arg("-V").output();
        if !probe.is_ok_and(|o| o.status.success()) {
            eprintln!("strace is not installed; skipping");
            return;
        }
        let result = run(Command::new("strace").args(["-f", "-o", "/dev/null", BINARY]).env_remove(ROOT_ENV));
        assert_eq!(result.status.code(), Some(3), "{}", stderr(&result));
        assert!(stderr(&result).contains("debugger"), "{}", stderr(&result));
    }
}

mod cpu {
    use super::*;

    #[test]
    fn arithmetic_and_flags() {
        let program = assemble(
            "
            MOVI R0, 5
            MOVI R1, 7
            ADD  R0, R1
            CMPI R0, 12
            JZ   ok
            HLT
        ok: MOVI R2, 1
            OUT  0x01, R2
            HLT
        ",
        )
        .unwrap();
        let mut machine = Machine::new();
        machine.load(&program.image, 0).unwrap();
        let out = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = out.clone();
        machine.attach_output(1, move |v| sink.borrow_mut().push(v));
        machine.run(5_000_000).unwrap();
        assert_eq!(*out.borrow(), vec![1]);
        assert_eq!(machine.registers[0], 12);
    }
}

mod pi {
    use super::*;

    #[test]
    fn all_algorithms_agree() {
        let digits = high_precision::pi_digits(500);
        assert!(digits.starts_with("31415926535897932384626433"));
        assert_eq!(digits, series::pi_digits(500));
        assert_eq!(digits, streaming::pi_digits(500));
    }
}

mod dependencies {
    use super::*;

    #[test]
    fn semver() {
        assert!(satisfies(&Version::parse("3.14.15").unwrap(), ">=3.14,<4").unwrap());
        assert!(satisfies(&Version::parse("2.3.9").unwrap(), "~2.3").unwrap());
        assert!(!satisfies(&Version::parse("3.1.4").unwrap(), ">=3.14").unwrap());
    }

    #[test]
    fn cycle_detected() {
        let a = Component::new("a", Version::parse("1.0.0").unwrap(), "x:y", &[("b", ">=1")]);
        let b = Component::new("b", Version::parse("1.0.0").unwrap(), "x:y", &[("a", ">=1")]);
        let error = resolve(&[a, b]).unwrap_err();
        assert_eq!(error.message, "dependency cycle among: a, b");
    }
}

mod peano {
    use super::*;

    #[test]
    fn length() {
        assert_eq!(to_int(&peano_length("hello world".chars())), 11);
    }
}
