Feature: theorem_file macro expansion
  The theorem_file proc macro should expand theorem files into deterministic
  per-file modules and harness stubs during compilation.

  Scenario: A valid theorem file compiles without Kani installed
    Given a fixture crate with one valid theorem file
    Then the fixture crate builds without a Kani dependency

  Scenario: A valid theorem file exposes a Kani proof harness when cargo-kani is installed
    Given a fixture crate with one valid theorem file
    Then cargo-kani lists the generated proof harness when installed

  Scenario: A multi-document theorem file compiles without Kani installed
    Given a fixture crate with one valid multi-document theorem file
    Then the fixture crate builds all generated theorem entries without a Kani dependency

  Scenario: An invalid theorem file fails compilation during macro expansion
    Given a fixture crate with one invalid theorem file
    Then compiling the fixture crate fails with an actionable theorem diagnostic

  Scenario: A theorem file without Kani evidence fails macro expansion
    Given a fixture crate with one theorem file missing Kani evidence
    Then compiling the fixture crate fails with a missing Kani evidence diagnostic

  Scenario: A multi-document theorem file with partial Kani evidence fails macro expansion
    Given a fixture crate with a multi-document theorem file missing one Kani evidence block
    Then compiling the fixture crate fails with the partial Kani evidence diagnostic
