
---

## **`Makefile`**

```makefile
# Makefile for PBFT Blockchain Project

# Project name
PROJECT_NAME = pbft-blockchain

# Cargo command
CARGO = cargo

# Default target
.PHONY: all
all: build

# Build the project
.PHONY: build
build:
	$(CARGO) build

# Run the primary node (node 0)
.PHONY: run-primary
run-primary:
	$(CARGO) run -- 0

# Run a replica node
.PHONY: run-replica
run-replica:
	@if [ -z "$(NODE_ID)" ]; then \
		echo "Usage: make run-replica NODE_ID=<node_id>"; \
	else \
		$(CARGO) run -- $(NODE_ID); \
	fi

# Run a Byzantine node
.PHONY: run-byzantine
run-byzantine:
	@if [ -z "$(NODE_ID)" ]; then \
		echo "Usage: make run-byzantine NODE_ID=<node_id>"; \
	else \
		$(CARGO) run -- $(NODE_ID) byzantine; \
	fi

# Run all nodes (example: 4 nodes)
.PHONY: run-all
run-all:
	@echo "Starting all nodes..."
	gnome-terminal -- bash -c "$(CARGO) run -- 0; exec bash"
	gnome-terminal -- bash -c "$(CARGO) run -- 1; exec bash"
	gnome-terminal -- bash -c "$(CARGO) run -- 2; exec bash"
	gnome-terminal -- bash -c "$(CARGO) run -- 3; exec bash"

# Clean generated files
.PHONY: clean
clean:
	$(CARGO) clean
	rm -f node_*.log node_*_state.json

# Display help information
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  make build            Build the project"
	@echo "  make run-primary      Run the primary node (node 0)"
	@echo "  make run-replica NODE_ID=<node_id>    Run a replica node"
	@echo "  make run-byzantine NODE_ID=<node_id>  Run a Byzantine node"
	@echo "  make run-all          Run all nodes (4 nodes)"
	@echo "  make clean            Clean generated files"
	@echo "  make help             Display this help information"
