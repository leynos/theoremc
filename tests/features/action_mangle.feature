Feature: Action name mangling
  To ensure deterministic, injective action resolution
  As a theorem compiler
  I want canonical action names to mangle into stable Rust identifiers

  Scenario: Simple action names produce correct mangled identifiers
    Given representative canonical action names
    Then each name produces the expected mangled identifier

  Scenario: Underscore escaping preserves injectivity
    Given action names that differ only in underscore placement
    Then their mangled identifiers are distinct

  Scenario: Mangled identifiers resolve to crate::theorem_actions
    Given a mangled canonical action name
    Then the resolution path begins with crate::theorem_actions
