//! Composition root for frieze.
//!
//! This is the one place (besides the gateway crates themselves) that
//! knows the concrete gateway types: it injects them into the
//! use-case interactors and hands the assembled value to a binary.
//! Binaries depend on this crate instead of wiring gateways
//! themselves, so adding another binary later reuses the same wiring,
//! and the use-case layer stays free of any gateway-crate knowledge.

use frieze_cargo::{CargoPackageResolver, CargoSchemasCollector};
use frieze_fs::{FsMetadataSource, FsOutputSink, FsPartialSource};
use frieze_usecase::GenerateOas;

/// Assembles the [`GenerateOas`] interactor with the production
/// gateways: cargo-backed package resolution and schema collection,
/// filesystem-backed configuration / partial / output handling.
///
/// The return type spells out the concrete gateway types on purpose —
/// this crate is where that knowledge is allowed to live. Callers
/// just run the returned interactor.
pub fn generate_oas() -> GenerateOas<
    CargoPackageResolver,
    FsMetadataSource,
    CargoSchemasCollector,
    FsPartialSource,
    FsOutputSink,
> {
    GenerateOas::new(
        CargoPackageResolver::new(),
        FsMetadataSource::new(),
        CargoSchemasCollector::new(),
        FsPartialSource::new(),
        FsOutputSink::new(),
    )
}
