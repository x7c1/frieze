//! `cargo-frieze` — the cargo subcommand front-end for frieze.
//!
//! The binary's only job is to parse the arguments, resolve the
//! target package from the current directory, obtain the assembled
//! interactor from `frieze-wire`, run it, and render the result:
//! one `generated → <path>` line per output on stdout, errors on
//! stderr with exit code 1. Cargo's own build log for the scratch
//! build streams through on stderr untouched.
//!
//! Argument handling is plain `std::env::args` matching — the surface
//! today is a single `generate` subcommand with no flags, which does
//! not yet justify an argument-parser dependency.

use std::process::ExitCode;

use frieze_model::PackageRoot;
use frieze_usecase::GenerateOasParams;

/// What the user asked for, parsed from `argv`.
enum Invocation {
    Generate,
    /// Anything unrecognized; carries the error to print above the
    /// usage message, when there is one.
    Usage {
        error: Option<String>,
    },
}

fn main() -> ExitCode {
    match parse_invocation(std::env::args().skip(1)) {
        Invocation::Generate => generate(),
        Invocation::Usage { error } => {
            if let Some(error) = &error {
                eprintln!("error: {error}");
                eprintln!();
            }
            eprintln!("Usage: cargo frieze generate");
            eprintln!();
            eprintln!("Generates the OAS documents declared under");
            eprintln!("[[package.metadata.frieze.outputs]] of the current package.");
            ExitCode::FAILURE
        }
    }
}

/// Parses the arguments after the binary name.
///
/// When cargo dispatches `cargo frieze ...` it invokes `cargo-frieze
/// frieze ...` — the subcommand name arrives as the first argument —
/// while a direct `cargo-frieze ...` invocation has no such prefix.
/// Both spellings are accepted by skipping a leading `frieze`.
fn parse_invocation(args: impl Iterator<Item = String>) -> Invocation {
    let mut args = args.peekable();
    if args.peek().map(String::as_str) == Some("frieze") {
        args.next();
    }
    match (args.next(), args.next()) {
        (Some(command), None) if command == "generate" => Invocation::Generate,
        (None, _) => Invocation::Usage { error: None },
        (Some(command), None) => Invocation::Usage {
            error: Some(format!("unknown command `{command}`")),
        },
        (Some(command), Some(extra)) => Invocation::Usage {
            error: Some(format!("unexpected argument `{extra}` after `{command}`")),
        },
    }
}

/// Runs the generate flow for the package in the current directory.
fn generate() -> ExitCode {
    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("error: cannot determine the current directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    let root = match PackageRoot::try_from_path(&current_dir) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("hint: run `cargo frieze generate` from a package directory");
            return ExitCode::FAILURE;
        }
    };
    let params = GenerateOasParams { root, filter: None };
    match frieze_wire::generate_oas().run(&params) {
        Ok(report) => {
            for written in &report.written {
                println!("generated → {}", written.path);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn accepts_generate_under_both_invocation_styles() {
        // `cargo frieze generate` → argv[1..] = ["frieze", "generate"]
        assert!(matches!(
            parse_invocation(args(&["frieze", "generate"])),
            Invocation::Generate
        ));
        // direct `cargo-frieze generate` → argv[1..] = ["generate"]
        assert!(matches!(
            parse_invocation(args(&["generate"])),
            Invocation::Generate
        ));
    }

    #[test]
    fn no_arguments_prints_plain_usage() {
        assert!(matches!(
            parse_invocation(args(&[])),
            Invocation::Usage { error: None }
        ));
        assert!(matches!(
            parse_invocation(args(&["frieze"])),
            Invocation::Usage { error: None }
        ));
    }

    #[test]
    fn unknown_commands_and_extra_arguments_are_reported() {
        assert!(matches!(
            parse_invocation(args(&["frieze", "genrate"])),
            Invocation::Usage { error: Some(message) } if message.contains("genrate")
        ));
        assert!(matches!(
            parse_invocation(args(&["frieze", "generate", "--check"])),
            Invocation::Usage { error: Some(message) } if message.contains("--check")
        ));
    }
}
