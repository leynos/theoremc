.PHONY: help all clean test build release lint lint-whitaker typecheck fmt check-fmt markdownlint nixie spelling check-adr-rfc-names test-workflow-contracts check-agent-native msrv

TARGET ?= cargo-theorem

export PATH := $(HOME)/.cargo/bin:$(HOME)/.bun/bin:$(PATH)

CARGO ?= cargo
WHITAKER ?= whitaker
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
RUSTDOC_FLAGS ?= -D warnings
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= --workspace $(CARGO_FLAGS) -- $(RUST_FLAGS)
NEXTEST_FLAGS ?= --workspace $(CARGO_FLAGS)
DOCTEST_FLAGS ?= --workspace --all-features --doc
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

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test test-workflow-contracts ## Perform a comprehensive check of code

clean: ## Remove build artefacts
	$(CARGO) clean

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(RUST_FLAGS) $(STANDARD_RUSTFLAGS)" $(CARGO) nextest run $(NEXTEST_FLAGS) $(BUILD_JOBS)
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(RUST_FLAGS) $(STANDARD_RUSTFLAGS)" $(CARGO) test $(DOCTEST_FLAGS) $(BUILD_JOBS)

target/%/$(TARGET): ## Build binary in debug or release mode
	$(if $(findstring release,$(@)),RUSTFLAGS="$${RUSTFLAGS-}",RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)") $(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release) -p cargo-theorem --bin $(TARGET)

lint: lint-whitaker ## Run rustdoc, Clippy and the Whitaker suite with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)" $(CARGO) doc --no-deps
	RUSTFLAGS="$${RUSTFLAGS:+$$RUSTFLAGS }$(STANDARD_RUSTFLAGS)" $(CARGO) clippy $(CLIPPY_FLAGS)

lint-whitaker: ## Run the Whitaker Dylint suite through its wrapper with warnings denied
	RUSTFLAGS= DYLINT_RUSTFLAGS="$(RUST_FLAGS)" $(WHITAKER) --all -- $(CARGO_FLAGS)

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
