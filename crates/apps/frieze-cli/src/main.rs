//! `cargo-frieze` — the cargo subcommand front-end for frieze.
//!
//! The binary's only job is to parse the arguments, obtain the
//! assembled interactor from `frieze-wire`, run it, and render the
//! result: one `generated → <path>` line per output on stdout (or
//! `up-to-date → <path>` under `--check`), errors and check
//! diagnostics on stderr with exit code 1. Cargo's own build log for
//! the scratch build streams through on stderr untouched. Which
//! package a run targets is the interactor's business — it resolves
//! the enclosing workspace from the current directory and honours the
//! `-p` request.
//!
//! Argument handling is plain `std::env::args` matching — the surface
//! today is a single `generate` subcommand with three optional flags,
//! which does not yet justify an argument-parser dependency.

use std::process::ExitCode;

use frieze_model::{OutputName, PackageName};
use frieze_usecase::{CheckOutcome, CheckedOutput, GenerateMode, GenerateOasParams, Report};

/// What the user asked for, parsed from `argv`.
enum Invocation {
    Generate {
        /// The value of `-p` / `--package`, verbatim; validated into a
        /// [`PackageName`] only when the command actually runs.
        package: Option<String>,
        /// The value of `--output`, verbatim; validated into an
        /// [`OutputName`] only when the command actually runs.
        output: Option<String>,
        /// Whether `--check` was passed: compare the existing output
        /// files instead of writing them.
        check: bool,
    },
    /// Anything unrecognized; carries the error to print above the
    /// usage message, when there is one.
    Usage { error: Option<String> },
}

fn main() -> ExitCode {
    match parse_invocation(std::env::args().skip(1)) {
        Invocation::Generate {
            package,
            output,
            check,
        } => generate(package, output, check),
        Invocation::Usage { error } => {
            if let Some(error) = &error {
                eprintln!("error: {error}");
                eprintln!();
            }
            eprintln!("Usage: cargo frieze generate [-p <package>] [--output <name>] [--check]");
            eprintln!();
            eprintln!("Generates the OAS documents declared under");
            eprintln!("[[package.metadata.frieze.outputs]] of the target package.");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  -p, --package <name>  Target the given workspace member instead of");
            eprintln!("                        the package resolved from the current directory");
            eprintln!("  --output <name>       Generate only the output declared under <name>");
            eprintln!("  --check               Write nothing; fail if any output file differs");
            eprintln!("                        from what a run without `--check` would write");
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
    match args.next() {
        None => Invocation::Usage { error: None },
        Some(command) if command == "generate" => parse_generate_args(args),
        Some(command) => Invocation::Usage {
            error: Some(format!("unknown command `{command}`")),
        },
    }
}

/// Parses the arguments after `generate`: at most one
/// `-p` / `--package` and at most one `--output` (each accepting both
/// the space-separated and the `=` spelling), an optional `--check`,
/// nothing else.
fn parse_generate_args(mut args: impl Iterator<Item = String>) -> Invocation {
    let mut package = None;
    let mut output = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        if arg == "--check" {
            if check {
                return Invocation::Usage {
                    error: Some("`--check` may be given at most once".to_string()),
                };
            }
            check = true;
            continue;
        }
        let parsed = parse_flag(&arg, &mut args);
        let (flag, value) = match parsed {
            Ok(pair) => pair,
            Err(error) => return Invocation::Usage { error: Some(error) },
        };
        let slot = match flag {
            Flag::Package => &mut package,
            Flag::Output => &mut output,
        };
        if slot.is_some() {
            return Invocation::Usage {
                error: Some(format!("`{}` may be given at most once", flag.long_name())),
            };
        }
        *slot = Some(value);
    }
    Invocation::Generate {
        package,
        output,
        check,
    }
}

/// The flags `generate` accepts.
#[derive(Clone, Copy)]
enum Flag {
    Package,
    Output,
}

impl Flag {
    fn long_name(self) -> &'static str {
        match self {
            Flag::Package => "--package",
            Flag::Output => "--output",
        }
    }
}

/// Matches one argument against the known flags, consuming the
/// following argument as the value for the space-separated spelling.
fn parse_flag(
    arg: &str,
    args: &mut impl Iterator<Item = String>,
) -> Result<(Flag, String), String> {
    for (flag, spellings) in [
        (Flag::Package, &["-p", "--package"][..]),
        (Flag::Output, &["--output"][..]),
    ] {
        for spelling in spellings {
            if arg == *spelling {
                return match args.next() {
                    Some(value) => Ok((flag, value)),
                    None => Err(format!("`{spelling}` requires a value")),
                };
            }
            if let Some(value) = arg
                .strip_prefix(spelling)
                .and_then(|rest| rest.strip_prefix('='))
            {
                return Ok((flag, value.to_string()));
            }
        }
    }
    Err(format!("unexpected argument `{arg}` after `generate`"))
}

/// Runs the generate flow for the resolved target package, optionally
/// pinned to the package named by `-p`, restricted to the output
/// named by `--output`, and switched to comparison by `--check`.
fn generate(package: Option<String>, output: Option<String>, check: bool) -> ExitCode {
    let package = match package.map(PackageName::new).transpose() {
        Ok(package) => package,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let filter = match output.map(OutputName::new).transpose() {
        Ok(filter) => filter,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let params = GenerateOasParams {
        package,
        filter,
        mode: if check {
            GenerateMode::Check
        } else {
            GenerateMode::Write
        },
    };
    match frieze_wire::generate_oas().run(&params) {
        Ok(Report::Written { outputs }) => {
            for record in &outputs {
                println!("generated → {}", record.path);
            }
            ExitCode::SUCCESS
        }
        Ok(Report::Checked { outcomes }) => render_check_report(&outcomes),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Renders a check run: passing outputs on stdout, one diagnostic per
/// failing output on stderr, plus a closing hint on how to fix them.
/// Exit code 1 as soon as a single output is not up to date.
fn render_check_report(outcomes: &[CheckedOutput]) -> ExitCode {
    for output in outcomes {
        match output.outcome {
            CheckOutcome::UpToDate => println!("up-to-date → {}", output.path),
            CheckOutcome::Stale | CheckOutcome::Missing => {
                eprintln!("error: {}", check_diagnosis(output));
            }
        }
    }
    let failed = outcomes
        .iter()
        .filter(|output| !output.is_up_to_date())
        .count();
    if failed == 0 {
        return ExitCode::SUCCESS;
    }
    eprintln!("error: {}", check_summary(failed));
    ExitCode::FAILURE
}

/// The per-output diagnostic line of a failed check.
fn check_diagnosis(output: &CheckedOutput) -> String {
    match output.outcome {
        CheckOutcome::UpToDate => unreachable!("up-to-date outputs are not diagnosed"),
        CheckOutcome::Stale => format!(
            "output `{}` is stale: `{}` does not match the generated document",
            output.name, output.path
        ),
        CheckOutcome::Missing => format!(
            "output `{}` is missing: `{}` does not exist",
            output.name, output.path
        ),
    }
}

/// The closing line of a failed check: what happened, and the exact
/// command that fixes it.
fn check_summary(failed: usize) -> String {
    let outputs = if failed == 1 {
        "output is"
    } else {
        "outputs are"
    };
    format!(
        "{failed} {outputs} not up to date: \
         run `cargo frieze generate` without `--check` to regenerate"
    )
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
            Invocation::Generate {
                package: None,
                output: None,
                check: false,
            }
        ));
        // direct `cargo-frieze generate` → argv[1..] = ["generate"]
        assert!(matches!(
            parse_invocation(args(&["generate"])),
            Invocation::Generate {
                package: None,
                output: None,
                check: false,
            }
        ));
    }

    #[test]
    fn accepts_the_output_flag_in_both_spellings() {
        assert!(matches!(
            parse_invocation(args(&["frieze", "generate", "--output", "public"])),
            Invocation::Generate { output: Some(name), .. } if name == "public"
        ));
        assert!(matches!(
            parse_invocation(args(&["generate", "--output=public"])),
            Invocation::Generate { output: Some(name), .. } if name == "public"
        ));
    }

    #[test]
    fn accepts_the_package_flag_in_every_spelling() {
        for spelling in [
            &["-p", "api"][..],
            &["--package", "api"][..],
            &["-p=api"][..],
            &["--package=api"][..],
        ] {
            let mut list = vec!["generate"];
            list.extend_from_slice(spelling);
            assert!(
                matches!(
                    parse_invocation(args(&list)),
                    Invocation::Generate { package: Some(name), .. } if name == "api"
                ),
                "spelling {spelling:?} should parse"
            );
        }
    }

    #[test]
    fn package_and_output_flags_compose() {
        assert!(matches!(
            parse_invocation(args(&["generate", "-p", "api", "--output", "public"])),
            Invocation::Generate {
                package: Some(package),
                output: Some(output),
                check: false,
            } if package == "api" && output == "public"
        ));
    }

    #[test]
    fn the_check_flag_composes_with_the_other_flags() {
        assert!(matches!(
            parse_invocation(args(&["generate", "--check"])),
            Invocation::Generate {
                package: None,
                output: None,
                check: true,
            }
        ));
        // Position does not matter.
        assert!(matches!(
            parse_invocation(args(&["generate", "--check", "-p", "api", "--output", "public"])),
            Invocation::Generate {
                package: Some(package),
                output: Some(output),
                check: true,
            } if package == "api" && output == "public"
        ));
    }

    #[test]
    fn check_flag_misuse_is_reported() {
        // A repeated flag.
        assert!(matches!(
            parse_invocation(args(&["generate", "--check", "--check"])),
            Invocation::Usage { error: Some(message) }
                if message == "`--check` may be given at most once"
        ));
        // `--check` takes no value.
        assert!(matches!(
            parse_invocation(args(&["generate", "--check=yes"])),
            Invocation::Usage { error: Some(message) }
                if message == "unexpected argument `--check=yes` after `generate`"
        ));
    }

    #[test]
    fn check_summary_names_the_fix_and_counts_correctly() {
        assert_eq!(
            check_summary(1),
            "1 output is not up to date: \
             run `cargo frieze generate` without `--check` to regenerate"
        );
        assert_eq!(
            check_summary(2),
            "2 outputs are not up to date: \
             run `cargo frieze generate` without `--check` to regenerate"
        );
    }

    #[test]
    fn output_flag_misuse_is_reported() {
        // A missing value.
        assert!(matches!(
            parse_invocation(args(&["generate", "--output"])),
            Invocation::Usage { error: Some(message) } if message.contains("requires a value")
        ));
        // A repeated flag.
        assert!(matches!(
            parse_invocation(args(&["generate", "--output", "a", "--output", "b"])),
            Invocation::Usage { error: Some(message) } if message.contains("at most once")
        ));
    }

    #[test]
    fn package_flag_misuse_is_reported() {
        // A missing value names the spelling that was used.
        assert!(matches!(
            parse_invocation(args(&["generate", "-p"])),
            Invocation::Usage { error: Some(message) } if message.contains("`-p` requires a value")
        ));
        // A repeated flag, across spellings.
        assert!(matches!(
            parse_invocation(args(&["generate", "-p", "a", "--package", "b"])),
            Invocation::Usage {
                error: Some(message),
            } if message.contains("`--package` may be given at most once")
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
            parse_invocation(args(&["frieze", "generate", "--verbose"])),
            Invocation::Usage { error: Some(message) } if message.contains("--verbose")
        ));
    }
}
