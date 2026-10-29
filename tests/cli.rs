use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hello-world"))
        .args(args)
        .output()
        .expect("failed to run binary")
}

#[test]
fn prints_hello_world() {
    let out = run(&[]);
    assert!(out.status.success());
    assert_eq!(out.stdout, b"hello world\n");
    assert!(out.stderr.is_empty());
}

#[test]
fn prints_version() {
    let out = run(&["--version"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "hello-world 6.7.0\n");
}

#[test]
fn rejects_unknown_argument() {
    let out = run(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
}
