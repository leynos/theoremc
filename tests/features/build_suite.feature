Feature: Build suite generation for theorem files

  Scenario: An empty crate still compiles with generated suite wiring
    Given a crate without a theorems directory
    Then the crate compiles successfully with the generated suite

  Scenario: A single theorem file is included automatically
    Given a crate with one theorem file
    Then the single theorem is included automatically and the crate compiles

  Scenario: Multiple theorem files compile in deterministic suite order
    Given a crate with multiple theorem files created in non-sorted order
    Then all theorems compile in deterministic suite order
