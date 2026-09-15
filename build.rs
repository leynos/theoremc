//! Cargo build-script entrypoint for theoremc build support.

fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
    let build_output = theoremc_build_support::prepare_build_script()?;

    println!("cargo::rustc-check-cfg=cfg(theoremc_has_theorems)");
    if build_output.has_theorems() {
        println!("cargo::rustc-cfg=theoremc_has_theorems");
    }
    for path in build_output.rerun_paths() {
        println!("cargo::rerun-if-changed={}", path.as_str());
    }

    Ok(())
}
