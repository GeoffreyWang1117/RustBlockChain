# PBFT Blockchain Implementation in Rust

## Introduction

This project is an implementation of the Practical Byzantine Fault Tolerance (PBFT) consensus algorithm in Rust. It allows a network of nodes to reach agreement (consensus) even in the presence of Byzantine faults (malicious or faulty nodes). The implementation focuses on safety and liveness properties, providing insights into building blockchain protocols using PBFT as a foundation.

## Table of Contents

- [Environment Requirements](#environment-requirements)
- [Project Structure](#project-structure)
- [Compilation and Execution](#compilation-and-execution)
  - [Compile the Project](#compile-the-project)
  - [Run Nodes](#run-nodes)
    - [Run the Primary Node (Node 0)](#run-the-primary-node-node-0)
    - [Run Replica Nodes](#run-replica-nodes)
    - [Run Byzantine Nodes](#run-byzantine-nodes)
  - [Run Example with Multiple Nodes](#run-example-with-multiple-nodes)
- [Testing Byzantine Nodes and View Changes](#testing-byzantine-nodes-and-view-changes)
  - [Simulate a Byzantine Node](#simulate-a-byzantine-node)
  - [Simulate Primary Node Failure](#simulate-primary-node-failure)
- [View Output Results](#view-output-results)
  - [Log Files](#log-files)
  - [Node State Files](#node-state-files)
  - [Adjust Log Level](#adjust-log-level)
- [Notes](#notes)
- [License](#license)

## Environment Requirements

- **Rust** programming language (version 1.50 or higher recommended)
- **Cargo** build tool
- **Operating System:** Linux or macOS recommended (for `Makefile` support)

## Project Structure

- `src/main.rs`: Program entry point; parses command-line arguments, initializes nodes, and starts execution.
- `src/node.rs`: Main logic of the node, including message handling, consensus process, and view changes.
- `src/message.rs`: Definitions of message types used in PBFT.
- `src/network.rs`: Simulated network communication between nodes.
- `src/config.rs`: Configuration parameters, such as the number of nodes `N` and the maximum number of Byzantine nodes `F`.
- `Cargo.toml`: Project dependencies and configuration.

## Compilation and Execution

### Compile the Project

In the project's root directory, run:

```bash
cargo build
```

## Run Nodes
### Run the Primary Node (Node 0)

```bash
cargo run -- 0
```

Or using the Makefile:

```bash
make run-primary
```

### Run Replica Nodes
```bash
cargo run -- <NODE_ID>
```
For example, to run node 1:

```bash
cargo run -- 1
```
Or using the Makefile:
```bash
make run-replica NODE_ID=1
```
### Run Byzantine Nodes
```bash
cargo run -- <NODE_ID> byzantine
```

For example, to run node 2 as a Byzantine node:

```bash
cargo run -- 2 byzantine
```
Or using the Makefile:

```bash
make run-byzantine NODE_ID=2
```
### Run Example with Multiple Nodes
To run an example with 4 nodes, you can open 4 terminal windows and run:

```bash
cargo run -- 0
cargo run -- 1
cargo run -- 2
cargo run -- 3
```
Or use the Makefile:

```bash
make run-all
```
Note: The make run-all command uses gnome-terminal to open new terminals on Linux systems. If you are using a different system or terminal emulator, you may need to modify the Makefile accordingly.

## Testing Byzantine Nodes and View Changes
### Simulate a Byzantine Node
To run node 2 as a Byzantine node:

```bash
cargo run -- 2 byzantine
```
### Simulate Primary Node Failure
After starting node 0 (the primary), you can manually close its terminal window to simulate a primary node failure. Other nodes will detect the timeout and initiate a view change.

## View Output Results
### Log Files
Each node generates a log file in the current directory with the format node_<NODE_ID>.log. You can view the log file using:

```bash
cat node_0.log
```
### Node State Files
The state of each node is saved in a file named node_<NODE_ID>_state.json, containing internal state information.

### Adjust Log Level
If you want to see detailed debug information, you can modify the log level in src/main.rs:

```rust
.filter(None, LevelFilter::Debug)
```
Then recompile and run the program.

## Notes
Number of Nodes: Ensure that the values of N and F in src/config.rs match the number of nodes you are running.
Sequential Node Startup: It is recommended to start nodes sequentially or with slight intervals to ensure the network module establishes connections properly.
Network Module: The network communication in this project is simulated. Further development is required to run in a real network environment.
## License
This project is licensed under the MIT License.