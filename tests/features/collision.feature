Feature: Mangled-identifier collision detection
  To prevent ambiguous code generation bindings
  As a theorem compiler
  I want mangled-identifier collisions detected before backend execution

  Scenario: Distinct action names across theorems are accepted
    Given a multi-theorem file with distinct action names
    Then loading succeeds without collision errors

  Scenario: Same action name reused within one theorem is accepted
    Given a single theorem with repeated action calls
    Then loading succeeds without collision errors

  Scenario: Distinct canonical names produce distinct mangled identifiers
    Given two canonical names that produce distinct mangled identifiers
    Then loading succeeds without collision errors for distinct names
