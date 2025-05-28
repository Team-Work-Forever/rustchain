# 🦀 RustChain: A project developed on FCUP - U.C SSD

RustChain is a public, non-permissioned blockchain implemented in Rust, designed to serve as a decentralized ledger for auction transactions. This project was developed as part of a master's degree program, focusing on secure, transparent, and distributed systems.

## 📚 Project Overview

The primary objective of RustChain is to create a decentralized platform that records auction transactions securely and transparently. By leveraging Rust's safety and concurrency features, the project aims to ensure high performance and reliability in a distributed environment.

## 🚀 Features

`Block Creation`: Implements the logic for creating and linking blocks containing auction data.

`Peer-to-Peer Networking`: Facilitates node discovery and communication between peers in the network.

`Consensus Mechanism`: Ensures agreement on the blockchain state across all nodes.

`Data Integrity`: Utilizes cryptographic techniques to maintain the integrity and immutability of auction records.

## 🛠️ Project Structure

The repository is organized into the following main components:

`blockchain/`: Contains modules related to block and transaction management.

`kademlia/`: Handles peer-to-peer networking functionalities, including node discovery and message propagation.

Makefile: Provides build and run commands for the project.

## 🧪 Getting Started

Prerequisites

Rust (latest stable version)

Cargo (Rust package manager)

Clone Repository

```bash
git clone https://github.com/Team-Work-Forever/rustchain.git rust-chain --depth 1
cd rust-chain
```

Build the project:

```bash
make build
```

Run the application:

```bash
make run node-1
```

```bash
make run node-2
```

```bash
make run node-3
```

```bash
make run node-4
```

```bash
make run node-5
```