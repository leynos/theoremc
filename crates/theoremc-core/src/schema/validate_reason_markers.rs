//! Stable validation reason strings shared by typed validation errors.

/// Message marker for invalid Kani `unwind`.
pub(crate) const KANI_UNWIND_NON_ZERO: &str =
    "Evidence.kani.unwind must be a positive integer (> 0)";
/// Message marker for required vacuity rationale.
pub(crate) const KANI_VACUITY_REASON_REQUIRED: &str =
    "vacuity_because is required when allow_vacuous is true";
/// Message marker for blank vacuity rationale.
pub(crate) const KANI_VACUITY_REASON_NON_EMPTY: &str =
    "Evidence.kani.vacuity_because must be non-empty after trimming";
