Feature: Cargo build discovery of theorem files
  Cargo should treat theorem files as real build inputs so theorem authors do
  not need to touch Rust sources or run cargo clean.

  Scenario: Existing theorem files are discovered recursively
    Given a crate with nested theorem files
    Then building twice stays fresh and editing a theorem reruns the build script

  Scenario: Non-theorem files do not participate in discovery
    Given a crate with ignored non-theorem files under theorems
    Then the build script emits only theorem inputs

  Scenario: Missing theorem directory is handled without manual seeding
    Given a crate without a theorems directory
    Then creating theorems later reruns the build script without manual seeding
