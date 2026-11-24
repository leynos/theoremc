.PHONY: help all clean fetch fixture-locks test build release lint lint-rustdoc lint-clippy lint-whitaker typecheck fmt check-fmt markdownlint nixie spelling test-workflow-contracts check-agent-native msrv check-adr-rfc-names install-kani kani

TARGET ?= cargo-theorem

export PATH := $(HOME)/.cargo/bin:$(HOME)/.bun/bin:$(PATH)

CARGO ?= cargo
WHITAKER ?= whitaker
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
RUSTDOC_FLAGS ?= -D warnings
# Every gate covers the whole workspace: the root `theoremc` package plus each
# member crate, including the root package's integration-test targets. The
# scope is named once so a gate cannot silently fall back to the root package,
# which is what Cargo selects when no package flag is given.
LINT_PACKAGES ?= --workspace
LINT_TARGET_FLAGS ?= --all-targets --all-features
CARGO_FLAGS ?= $(LINT_PACKAGES) $(LINT_TARGET_FLAGS)
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
# `cargo doc` rejects `--all-targets`, so documentation carries its own flags.
RUSTDOC_TARGET_FLAGS ?= $(LINT_PACKAGES) --all-features --no-deps
NEXTEST_FLAGS ?= $(CARGO_FLAGS)
DOCTEST_FLAGS ?= $(LINT_PACKAGES) --all-features --doc
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie
# `make fmt` and `make check-fmt` call mdtablefix directly. `--git` selects the
# Markdown files Git tracks and `--include-untracked` adds the untracked files
# Git does not ignore, so a new document is formatted before it is staged.
# Both modes need mdtablefix 0.6.0 or later; CI pins the version in
# .github/workflows/ci.yml.
MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences

UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools
# The CV-005 CodeScene contracts live in shared-actions and run from a full
# commit, so a fix is a pin bump. `.github/cv005.toml` holds this repository's
# only parameters.
CV005_CONTRACTS_REF ?= 88977798a5c3bae1549afb99642529488c665276
CV005_CONTRACTS = $(UV_ENV) $(UV) tool run --python 3.13 \
	--from 'git+https://github.com/leynos/shared-actions@$(CV005_CONTRACTS_REF)\#subdirectory=packages/cv005-contracts' \
	cv005-contracts

# The development build standard (concordat rule `rust-build-defaults`):
# the parallel rustc frontend and, on Linux, the mold linker. An assigned
# RUSTFLAGS replaces every `rustflags` table in .cargo/config.toml, so each
# recipe that sets it composes these onto any inherited value (CI's
# setup-rust exports one), except coverage, which stays on LLVM and the
# platform linker.
BUILD_HOST_OS := $(shell uname -s)
STANDARD_RUSTFLAGS := -Zthreads=8$(if $(filter Linux,$(BUILD_HOST_OS)), -Clink-arg=-fuse-ld=mold)

test-workflow-contracts: ## Check the CV-005 CodeScene workflow contracts
	$(CV005_CONTRACTS) check --repository .

check-agent-native: ## Check OrthoConfig agent-native command policy
	cargo orthohelp --check-agent-native --package cargo-theorem

msrv: ## Check the workspace minimum supported Rust version
	RUSTFLAGS="" $(CARGO) +1.89 check --workspace --all-targets --all-features --locked

# The shared rust-prover-tools package owns Kani acquisition. CI installs it
# through the shared install-prover-tools action; `install-kani` is the local
# equivalent. The package revision is read from one file, and the Kani release
# from tools/kani/VERSION.
override PROVER_TOOLS_REF := $(strip $(shell awk '/^ref:/ { print $$2; exit }' tools/rust-prover-tools/REF))
PROVER_TOOLS ?= $(UV) tool run --python 3.14 \
	--from git+https://github.com/leynos/rust-prover-tools@$(PROVER_TOOLS_REF) prover-tools
# The prebuilt Kani covers Linux x86_64 only, so any other host builds it.
KANI_HOST_KERNEL := $(shell uname -s)
KANI_HOST_MACHINE := $(shell uname -m)
KANI_INSTALL_FLAGS ?= $(if $(and $(filter Linux,$(KANI_HOST_KERNEL)),$(filter x86_64,$(KANI_HOST_MACHINE))),,--compile)

install-kani: ## Install the pinned Kani release (front-end in CARGO_HOME/bin, bundle in KANI_HOME)
	$(PROVER_TOOLS) kani install --repo-root . $(KANI_INSTALL_FLAGS)

# The Kani scenarios of the macro behavioural suite expand `theorem_file!` in a
# fixture crate and run `cargo kani` over the generated harnesses. With
# THEOREMC_REQUIRE_KANI=1 a missing or unusable Kani fails them instead of
# skipping, so an emission Kani rejects, or a runner without Kani, is a red job.
# The two scenarios are named exactly, and the recipe requires that both ran: a
# renamed scenario would otherwise match nothing and pass with zero tests.
override KANI_SCENARIOS := kani_lists_the_generated_proof_harness kani_verifies_every_generated_harness
# The scenarios build the macro fixture `--frozen`, so they need the fixture
# Cargo home populated exactly as `make test` does; `fetch` is its prerequisite
# for that reason. The home is exported so the fixture build and the fetch step
# cannot disagree about which directory holds the crates.
kani: fetch ## Verify the generated theorem harnesses with cargo kani (requires an installed Kani)
	@out=$$(THEOREMC_REQUIRE_KANI=1 THEOREMC_FIXTURE_CARGO_HOME="$(FIXTURE_CARGO_HOME)" RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(RUST_FLAGS)" $(CARGO) test --test theorem_file_macro_bdd $(BUILD_JOBS) -- --exact --test-threads=1 $(KANI_SCENARIOS) 2>&1); \
	status=$$?; printf '%s\n' "$$out"; \
	[ "$$status" -eq 0 ] || exit "$$status"; \
	for scenario in $(KANI_SCENARIOS); do \
		printf '%s\n' "$$out" | grep -qF "test $$scenario ... ok" || { echo "kani: scenario $$scenario did not run" >&2; exit 1; }; \
	done

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test test-workflow-contracts ## Perform a comprehensive check of code

clean: ## Remove build artefacts
	$(CARGO) clean

# The fixture crates the BDD suites build get their own Cargo home, because
# offline Cargo still takes `$CARGO_HOME/.package-cache` exclusively: a nested
# build sharing the caller's home waits behind any other `cargo` on the
# machine. Nothing else uses this home, so it sits inside the target directory
# and `make clean` removes it with the rest of the build output. `make test`
# exports the resolved path under the name the fixture helpers read.
FIXTURE_CARGO_HOME ?= $(CURDIR)/target/theoremc-fixture-cargo-home
# One committed template per fixture dependency graph. `fixture-locks`
# regenerates each one's lock; `tests/fixture_lock_contract.rs` holds them to
# the workspace lock (concordat NC-001).
FIXTURE_MANIFESTS := \
	tests/fixtures/build_support_fixture/Cargo.toml \
	tests/fixtures/theorem_file_macro_fixture/Cargo.toml

# Fetch once, then frozen. Every nested cargo these tests start runs
# `--frozen`, so a package the fetch missed fails at once with Cargo's offline
# diagnostic instead of resolving from the network while holding a lock. Add a
# manifest here when a fixture gains one: the failure names the crate, and the
# fix is this list.
fetch: ## Fetch every dependency the workspace and the fixture crates need
	$(CARGO) fetch --locked
	@set -e; for manifest in $(FIXTURE_MANIFESTS); do \
		echo "  fetching $$manifest into $(FIXTURE_CARGO_HOME)"; \
		CARGO_HOME="$(FIXTURE_CARGO_HOME)" $(CARGO) fetch --locked --manifest-path "$$manifest"; \
	done

# Copies the workspace lock into each template and prunes it to the graph that
# template declares. The pruning runs offline and only removes entries, so the
# versions the workspace resolved are preserved; a template that resolves a
# different version would fail `tests/fixture_lock_contract.rs`.
fixture-locks: ## Regenerate the committed fixture-template locks from the workspace lock
	@set -e; for manifest in $(FIXTURE_MANIFESTS); do \
		lock=$$(dirname "$$manifest")/Cargo.lock; \
		echo "  reseeding $$lock"; \
		cp Cargo.lock "$$lock"; \
		$(CARGO) update --workspace --offline --manifest-path "$$manifest" >/dev/null; \
	done

# `fetch` is a prerequisite rather than a recipe line so `make -j test` still
# completes it before any test starts. A sub-make would also work, but under
# `make -n` GNU make runs sub-makes for real, and the build-standard contract
# reads this target with `make -n`; a prerequisite is printed and not run.
test: fetch ## Run tests with warnings treated as errors
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(RUST_FLAGS) $(STANDARD_RUSTFLAGS)" THEOREMC_FIXTURE_CARGO_HOME="$(FIXTURE_CARGO_HOME)" $(CARGO) nextest run $(NEXTEST_FLAGS) $(BUILD_JOBS)
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(RUST_FLAGS) $(STANDARD_RUSTFLAGS)" THEOREMC_FIXTURE_CARGO_HOME="$(FIXTURE_CARGO_HOME)" $(CARGO) test $(DOCTEST_FLAGS) $(BUILD_JOBS)

target/%/$(TARGET): ## Build binary in debug or release mode
	$(if $(findstring release,$(@)),RUSTFLAGS="$${RUSTFLAGS-}",RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)") $(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release) -p cargo-theorem --bin $(TARGET)

# A prerequisite list is not a sequencing guarantee: under `make -j lint` the
# three stages would start at once and queue against one another for compiler
# slots. Recursing through `$(MAKE)` gives each stage its own recipe line, so
# they run in the listed order even under `-j`, and a failing stage stops the
# aggregate with that stage's exit status.
lint: ## Run rustdoc, Clippy and the Whitaker suite over the workspace, in order
	$(MAKE) lint-rustdoc
	$(MAKE) lint-clippy
	$(MAKE) lint-whitaker

lint-rustdoc: ## Build rustdoc for every workspace package with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)" $(CARGO) doc $(RUSTDOC_TARGET_FLAGS)

lint-clippy: ## Lint every workspace package, target and feature with warnings denied
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)" $(CARGO) clippy $(CLIPPY_FLAGS)

# `--workspace` is Whitaker's own flag, and Whitaker forwards it to the
# `cargo check` it runs. Passing it after `--` as well makes Cargo reject the
# run as a duplicate, so the forwarded arguments carry only the target and
# feature flags.
lint-whitaker: ## Run the Whitaker Dylint suite over the workspace with warnings denied
	RUSTFLAGS= DYLINT_RUSTFLAGS="$(RUST_FLAGS)" $(WHITAKER) --all $(LINT_PACKAGES) -- $(LINT_TARGET_FLAGS)

typecheck: ## Run type checking for all targets and features
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)" $(CARGO) check $(CARGO_FLAGS)

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	$(MDTABLEFIX) --in-place $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)
	@unset FORCE_COLOR; $(MDLINT) --fix "**/*.md"

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check
	$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

# The spelling gate's pinned builder. The defaults are the adopted baseline;
# the overrides let a contributor point the same recipe at another builder
# revision, repository, or scope without editing it.
SPELLING_BUILDER ?= git+https://github.com/leynos/typos-config-builder.git
SPELLING_BUILDER_REF ?= v0.1.3
SPELLING_REPOSITORY ?= .
SPELLING_SCOPE ?= all
SPELLING_COMMAND = $(UV_ENV) "$(UV)" tool run --python 3.14 \
	--from "$(SPELLING_BUILDER)@$(SPELLING_BUILDER_REF)" \
	typos-config-builder gate --repository "$(SPELLING_REPOSITORY)" --scope $(SPELLING_SCOPE)

spelling: ## Check spelling and shared phrase corrections
	$(SPELLING_COMMAND)

check-adr-rfc-names: ## Check ADR/RFC filenames, serials, titles, and local links
	$(UV_ENV) $(UV) run --python 3.13 --with 'pytest==8.4.2' --with 'hypothesis==6.170.15' -- python -B -m pytest tools/test_check_adr_rfc_names.py tools/test_adr_rfc_properties.py tools/test_adr_rfc_command.py
	$(UV_ENV) $(UV) run --python 3.13 -- python -B tools/check_adr_rfc_names.py

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
