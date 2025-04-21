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
	@echo "  gen-proto			- Generate proto files"
	@echo "  build				- Build the default binary (ledger) or a specified binary"
	@echo "  build bin=<binary_name>	- Build a specific binary (e.g., make build bin=p2p)"
	@echo "  build-all			- Build all binaries"
	@echo "  run				- Run the default binary (ledger)"
	@echo "  run bin=<binary_name>		- Run a specific binary (e.g., make run bin=p2p)"

.PHONY: gen-proto
gen-proto:
	echo "hello world"

.PHONY: build
build:
	@if [ -z "$(bin)" ]; then \
		cargo build --bin $(DEFAULT_BIN); \
	else \
		cargo build --bin $(bin); \
	fi

.PHONY: build-all
build-all:
	@echo "Building all binaries..."
	@cargo build --all

.PHONY: run
run:
	@if [ -z "$(bin)" ]; then \
		cargo run --bin $(DEFAULT_BIN); \
	else \
		cargo run --bin $(bin); \
	fi
