Feature: Per-file module naming
  To ensure deterministic, collision-resistant per-file Rust module names
  As a theorem compiler
  I want .theorem file paths to mangle into stable module identifiers

  Scenario: Simple paths produce deterministic module names
    Given representative .theorem file paths
    Then each path produces the expected module name

  Scenario: Mixed separators produce stable human-recognizable names
    Given paths with forward slashes and backslashes
    Then the mangled stems are identical but module names differ

  Scenario: Punctuation-heavy paths are disambiguated by hash
    Given paths that differ only in punctuation
    Then their module names are distinct because hash suffixes differ
