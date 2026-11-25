//! # plat: the Platinum command-line tool
//!
//! A thin layer over the compiler crates. Its job is to read arguments, call
//! into `plat-syntax` (and later phases), and hand the resulting diagnostics to
//! a renderer: JSON when `--json` is passed, human-readable otherwise.
//!
//! Exit codes (spec/09-tooling.md): 0 success, 1 diagnostics with errors,
//! 2 usage error, 101 program panic.
//!
//! This is the M0 step 1 placeholder. Real subcommands (`check`, `explain`)
//! arrive in step 11 using `clap`.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("plat {}: no commands yet (M0 in progress)", env!("CARGO_PKG_VERSION"));
    ExitCode::from(2)
}
