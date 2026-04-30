Feature: theorem_file macro expansion
  The theorem_file proc macro should expand theorem files into deterministic
  per-file modules and harness stubs during compilation.

  Scenario: A valid theorem file produces the expected generated symbol paths
    Given a fixture crate with one valid theorem file
    Then the fixture crate tests can refer to the generated private symbols

  Scenario: A multi-document theorem file generates one harness stub per document
    Given a fixture crate with one valid multi-document theorem file
    Then the fixture crate tests can refer to all generated harness stubs

  Scenario: An invalid theorem file fails compilation during macro expansion
    Given a fixture crate with one invalid theorem file
    Then compiling the fixture crate fails with an actionable theorem diagnostic
