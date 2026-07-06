//! `frieze-cli` — thin driver skeleton for the generate flow.
//!
//! The binary's only job is to parse arguments into the parsed domain
//! types, obtain the assembled interactor from `frieze-wire`, run it,
//! and render errors. None of that behavior exists yet — argument
//! parsing and the run land together with the concrete gateway
//! implementations. For now the skeleton proves the wiring compiles
//! end to end and exits with a clear not-implemented message.

use std::process::ExitCode;

fn main() -> ExitCode {
    // Assembling the interactor is side-effect free; running it is
    // what will execute the generate flow once the gateways are
    // implemented.
    let _interactor = frieze_wire::generate_oas();
    eprintln!("frieze: the generate flow is not implemented yet");
    ExitCode::FAILURE
}
