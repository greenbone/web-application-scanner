.PHONY: test build clean build-release run install lint check-format format check-compile docs coverage lint-deps check-unused-deps

INSTALL_PREFIX ?= /usr/local

test:
	cargo test --verbose --locked --all-targets

build:
	cargo build --verbose --locked

build-release:
	cargo build --verbose --locked --release

clean:
	cargo clean

install:
	cargo install --path . --locked --root $(DESTDIR)$(INSTALL_PREFIX)

lint:
	cargo clippy --locked --all-targets -- -D warnings

check-format:
	cargo fmt --all -- --check

check-compile:
	cargo check --all-targets --locked

format:
	cargo fmt --all

docs:
	cargo doc --locked --no-deps

coverage:
	cargo llvm-cov --locked --all-targets --html --output-dir target/coverage/html
	cargo llvm-cov report --locked --lcov --output-path target/coverage/lcov.info

lint-deps:
	cargo deny --locked check

check-unused-deps:
	cargo machete
