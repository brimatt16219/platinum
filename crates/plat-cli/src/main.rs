//! # plat: the Platinum command-line tool
//!
//! A thin layer over the compiler crates. Its job is to read arguments, call
//! into the compiler (only the lexer, as of M0), and hand the resulting
//! diagnostics to a renderer: JSON on stdout with `--json`, human-readable
//! reports on stderr otherwise.
//!
//! ## Argument parsing with clap
//!
//! We describe the command line as plain Rust types (`Cli` and `Command`)
//! and `#[derive(Parser)]` generates the parser, the `--help` text (from the
//! `///` doc comments), and error messages for bad arguments. Each enum
//! variant of `Command` is one subcommand, and its fields are that
//! subcommand's arguments. This is the same idea as attribute-based model
//! binding in ASP.NET: declare the shape, let the framework fill it in.
//!
//! ## Contract (spec/09-tooling.md)
//!
//! - Every command accepts `--json`. In JSON mode stdout carries exactly one
//!   JSON document and nothing else; human-facing messages go to stderr.
//! - Exit codes: 0 success, 1 diagnostics with errors, 2 usage error (bad
//!   arguments, unreadable file, unknown code), 101 program panic (M1+).

use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod check;
mod explain;

/// Exit code for a usage error: bad arguments, unreadable input, unknown code.
/// (clap also exits with 2 for argument errors it detects itself.)
pub const EXIT_USAGE: u8 = 2;

/// The Platinum compiler and tools.
#[derive(Parser)]
// The `///` comment above becomes the `--help` description.
#[command(name = "plat", version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print machine-readable JSON on stdout instead of human output.
    #[arg(long, global = true)]
    json: bool,

    /// When to use colors in human output.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,
}

#[derive(Subcommand)]
enum Command {
    /// Check files for errors without running them (M0: lexical errors only).
    Check {
        /// The `.plat` files to check.
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,
    },
    /// Show the long explanation for a diagnostic code, such as PT0002.
    Explain {
        /// The code to explain, for example PT0002.
        code: String,
    },
    /// Run a program (arrives in M1).
    Run {
        /// The `.plat` file to run.
        file: std::path::PathBuf,
    },
    /// Format source files (arrives in M1).
    Fmt {
        /// The `.plat` files to format.
        files: Vec<std::path::PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorChoice {
    /// Color when writing to a terminal and NO_COLOR is not set.
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    fn enabled(self) -> bool {
        use std::io::IsTerminal;
        match self {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            // https://no-color.org: any non-empty NO_COLOR disables color.
            ColorChoice::Auto => {
                let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
                std::io::stderr().is_terminal() && !no_color
            }
        }
    }
}

fn main() -> ExitCode {
    // On bad arguments `parse` prints a message and exits with code 2.
    let cli = Cli::parse();
    let color = cli.color.enabled();

    match &cli.command {
        Command::Check { files } => check::run(files, cli.json, color),
        Command::Explain { code } => explain::run(code, cli.json),
        Command::Run { .. } => not_yet("run", "M1"),
        Command::Fmt { .. } => not_yet("fmt", "M1"),
    }
}

fn not_yet(command: &str, milestone: &str) -> ExitCode {
    eprintln!("error: `plat {command}` is not implemented yet (it arrives in {milestone})");
    ExitCode::from(EXIT_USAGE)
}
