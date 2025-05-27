DEFAULT_BIN=ledger

# Load .env variables file
ifneq (,$(wildcard ./.env))
    include .env
    export
endif

.PHONY: help
help:
	@echo "Makefile for managing Project tools"
	@echo
	@echo "Actions:"
	@echo "  run-bootstrap-1			- Generate proto files"
	@echo "  run-bootstrap-2			- Generate proto files"
	@echo "  run-client-1				- Run client 1"
	@echo "  run-client-2				- Run clinet 2"
	@echo "  run-client-3				- Run clinet 3"

.PHONY: run-test-1
run-test-1:
	@cargo run --bin p2p -- --config config/test1.toml

.PHONY: run-test-c
run-test-c:
	@cargo run --bin p2p -- --config config/testc.toml

.PHONY: run-test-another
run-test-another:
	@cargo run --bin p2p -- --config config/test-another.toml

.PHONY: run-test-2
run-test-2:
	@cargo run --bin p2p -- --config config/test2.toml

.PHONY: run-bootstrap-1
run-bootstrap-1:
	@cargo run --bin p2p -- --config config/boot-1.toml

.PHONY: run-bootstrap-2
run-bootstrap-2:
	@cargo run --bin p2p -- --config config/boot-2.toml

.PHONY: run-bootstrap-3
run-bootstrap-3:
	@cargo run --bin p2p -- --config config/boot-3.toml

.PHONY: run-client-1
run-client-1:
	@cargo run --bin p2p -- --config config/client-1.toml

.PHONY: run-client-2
run-client-2:
	@cargo run --bin p2p -- --config config/client-2.toml

.PHONY: run-client-3
run-client-3:
	@cargo run --bin p2p -- --config config/client-3.toml

.PHONY: run-client-4
run-client-4:
	@cargo run --bin p2p -- --config config/client-4.toml

# .PHONY: gen-proto
# gen-proto:
# 	echo "hello world"

# .PHONY: build
# build:
# 	@if [ -z "$(bin)" ]; then \
# 		cargo build --bin $(DEFAULT_BIN); \
# 	else \
# 		cargo build --bin $(bin); \
# 	fi

# .PHONY: build-all
# build-all:
# 	@echo "Building all binaries..."
# 	@cargo build --all

# .PHONY: run
# run:
# 	@if [ -z "$(bin)" ]; then \
# 		cargo run --bin $(DEFAULT_BIN); \
# 	else \
# 		cargo run --bin $(bin); \
# 	fi
