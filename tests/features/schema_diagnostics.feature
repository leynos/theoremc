Feature: Structured schema diagnostics
  To make parser and validator failures actionable
  As a theorem author
  I want source-located diagnostics with stable output shape

  Scenario: Parser failures include explicit source and location
    Given a parser-invalid theorem fixture
    Then loading fails with source-located parser diagnostics

  Scenario: Validator failures include explicit source and location
    Given a validator-invalid theorem fixture
    Then loading fails with source-located validator diagnostics

  Scenario: Valid fixtures still parse when source is supplied
    Given a valid theorem fixture for diagnostics
    Then loading succeeds with explicit source
