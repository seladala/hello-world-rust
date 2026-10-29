use std::process::ExitCode;

const GREETING: &str = "hello world";

fn main() -> ExitCode {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("usage: hello-world [-h] [--version]\n\nPrints hello world.");
                return ExitCode::SUCCESS;
            }
            "--version" => {
                println!("hello-world {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("hello-world: unrecognized argument: {other}");
                return ExitCode::from(2);
            }
        }
    }

    println!("{GREETING}");
    ExitCode::SUCCESS
}
