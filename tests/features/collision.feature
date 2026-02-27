Feature: Action name collision detection
  To prevent ambiguous code generation bindings
  As a theorem compiler
  I want duplicate action names detected before backend execution

  Scenario: Distinct action names across theorems are accepted
    Given a multi-theorem file with distinct action names
    Then loading succeeds without collision errors

  Scenario: Same action name reused within one theorem is accepted
    Given a single theorem with repeated action calls
    Then loading succeeds without collision errors

  Scenario: Mangled identifier collision is detected
    Given two canonical names that produce the same mangled identifier
    Then the collision is reported with both canonical names
