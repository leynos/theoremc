//! Stable validation reason fragments shared with location mapping logic.

/// Message marker for blank `About`.
pub(crate) const ABOUT_NON_EMPTY: &str = "About must be non-empty after trimming";
/// Section marker for indexed `Prove` failures.
pub(crate) const PROVE_ASSERTION: &str = "Prove assertion";
/// Section marker for indexed `Assume` failures.
pub(crate) const ASSUME_CONSTRAINT: &str = "Assume constraint";
/// Section marker for indexed `Witness` failures.
pub(crate) const WITNESS: &str = "Witness";
/// Fragment used when a failure concerns the `because` field.
pub(crate) const BECAUSE_FIELD_FRAGMENT: &str = ": because ";
/// Message marker for invalid Kani `unwind`.
pub(crate) const KANI_UNWIND_NON_ZERO: &str =
    "Evidence.kani.unwind must be a positive integer (> 0)";
/// Message marker for required vacuity rationale.
pub(crate) const KANI_VACUITY_REASON_REQUIRED: &str =
    "vacuity_because is required when allow_vacuous is true";
/// Message marker for blank vacuity rationale.
pub(crate) const KANI_VACUITY_REASON_NON_EMPTY: &str =
    "Evidence.kani.vacuity_because must be non-empty after trimming";
