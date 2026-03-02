Feature: Argument value decoding
  To ensure theorem argument values have stable, explicit semantics
  As a theorem author
  I want plain YAML strings treated as literals and variable
  references to require explicit { ref: name } wrappers

  Scenario: Plain string arguments are decoded as literals
    Given a theorem file with plain string arguments
    Then loading succeeds and arguments are string literals

  Scenario: Explicit ref arguments are decoded as references
    Given a theorem file with explicit ref arguments
    Then loading succeeds and arguments are variable references

  Scenario: Integer and boolean arguments are decoded as literals
    Given a theorem file with integer and boolean arguments
    Then loading succeeds and arguments are scalar literals

  Scenario: Invalid ref target is rejected
    Given a theorem file with an invalid ref target
    Then loading fails with an actionable error message

  Scenario: Adding a binding cannot alter literal argument semantics
    Given a theorem with a plain string argument matching a binding name
    Then the argument remains a string literal regardless of bindings
