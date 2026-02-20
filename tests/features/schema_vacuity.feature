Feature: Schema vacuity policy
  Theorem documents should reject vacuous defaults unless explicitly justified.

  Scenario: Default policy accepts witness-backed theorem
    Given a valid default theorem fixture
    Then the default theorem fixture loads successfully

  Scenario: Vacuous override accepts theorem with reason
    Given a valid vacuous theorem fixture
    Then the vacuous theorem fixture loads successfully

  Scenario: Default policy rejects missing witness
    Given a default theorem fixture missing witness
    Then loading fails because witness is required by default

  Scenario: Explicit non-vacuous policy rejects missing witness
    Given an explicit non-vacuous theorem fixture missing witness
    Then loading fails because witness is required when non-vacuous is explicit

  Scenario: Vacuous override rejects missing reason
    Given a vacuous theorem fixture without vacuity reason
    Then loading fails because vacuity reason is required

  Scenario: Vacuous override rejects blank reason
    Given a vacuous theorem fixture with blank vacuity reason
    Then loading fails because vacuity reason is blank
