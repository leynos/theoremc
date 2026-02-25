Feature: Canonical action-name validation
  To keep action resolution deterministic and compile-time checkable
  As a theorem author
  I want canonical dotted action names with non-keyword segments

  Scenario: Canonical action names are accepted
    Given a theorem fixture with canonical action names
    Then loading succeeds for canonical action names

  Scenario: Malformed canonical action names are rejected
    Given a theorem fixture with malformed action names
    Then loading fails for malformed action names

  Scenario: Reserved keyword action segments are rejected
    Given a theorem fixture with keyword action segments
    Then loading fails for keyword action segments
