Feature: Deterministic theorem harness naming
  To generate stable Kani proof harness symbols
  As a theorem compiler
  I want theorem identifiers and theorem keys mangled deterministically

  Scenario: Representative theorem identifiers produce deterministic harness names
    Given representative theorem paths and theorem identifiers
    Then each theorem produces the expected harness identifier

  Scenario: Theorem slugs preserve snake-case identifiers
    Given theorem identifiers that are already snake case
    Then the harness slug stays unchanged

  Scenario: Duplicate theorem keys are rejected during loading
    Given a multi-document theorem source with duplicate theorem identifiers
    Then loading fails with a duplicate theorem key diagnostic
