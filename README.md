# ForgeIO / OpenForge

> An open-source, Rust-based SCADA & Machine Interface platform

ForgeIO (also known as OpenForge) leverages Rust’s performance, safety, and concurrency to deliver a modern solution for industrial automation, including a comprehensive visual design environment and edge deployment capabilities.

---

## Table of Contents

1. [Overview](#overview)
2. [Key Features](#key-features)
3. [Architecture](#architecture)
4. [Roadmap](#roadmap)
5. [Technology Stack](#technology-stack)
6. [Getting Started](#getting-started)
7. [Contributing](#contributing)
8. [License](#license)

---

## Overview

ForgeIO / OpenForge is designed to:

* **Collect** data from industrial devices via OPC UA, Modbus, and MQTT
* **Manage** real-time tags and store historical data
* **Script** custom logic in Python (via PyO3) or Rust
* **Visualize** and **control** processes through a web-based drag-and-drop interface
* **Deploy** at the edge or in centralized gateways
* **Extend** and **collaborate** via open-source development

---

## Key Features

* **Data Acquisition Layer**: Native support for OPC UA, Modbus TCP/RTU, and MQTT
* **Tag System & Historian**: Thread-safe in-memory tags, seamless time-series storage
* **Embedded Scripting**: Python 3+ integration for event handlers and calculations
* **Real-Time Events & Alarms**: Configurable triggers, notifications, and workflows
* **Visual Design Environment**: Intuitive drag-and-drop editor, script console, and project manager
* **Edge & Machine Interface**: Local buffering, protocol translation, and responsive Rust/WASM UIs
* **Version Control**: Git-native project storage and history

---

## Architecture

### Gateway Server

* **Data Acquisition**: OPC UA, Modbus, MQTT modules
* **Scripting Engine**: Embedded Python via PyO3
* **Tag Management**: Fast, concurrent tag reads/writes
* **Event Processing**: Alarm management and notification services
* **API Layer**: WebSockets, REST, gRPC endpoints

### Edge & Machine Interface Layer

* **Protocol Gateways**: Local OPC UA and MQTT bridges
* **Visualization UI**: Rust/WASM components for HMI screens
* **Local Historian**: Temporary storage to support offline operation

### Visual Design Environment

* **Drag-and-Drop Interface Designer**
* **Project Configuration Tools**
* **Script Editor** (Python & Rust)
* **Tag/Historian Configuration**
* **Git Integration**

---

## Roadmap

### Phase 1: Core Platform

* Implement data acquisition and tag management modules
* Integrate Python scripting engine
* Set up basic REST and WebSocket APIs

### Phase 2: Visual Design Environment

* Develop drag-and-drop UI builder
* Add project management and configuration tools
* Create integrated script editor and live preview

### Phase 3: Advanced Capabilities

* Enhance real-time synchronization and performance
* Implement full Git-based version control features
* Build testing framework and simulation environment

---

## Technology Stack

### Backend (Rust)

* **Frameworks**: Axum, Tokio
* **Libraries**: tokio-modbus, opcua, rumqttc
* **Scripting**: PyO3
* **Storage**: SQLx, TimescaleDB
* **Realtime**: tokio-tungstenite (WebSockets)

### Frontend (Rust/WASM)

* **UI**: Leptos or Yew
* **Compilation**: WebAssembly for high performance
* **Components**: Custom drag-and-drop toolkit

---

## Getting Started

1. **Prerequisites**

   * Rust `>=1.60`
   * Python `>=3.8`
   * Cargo, Git
2. **Clone the repo**

   ```bash
   git clone https://github.com/your-org/forgeio.git  # or openforge
   cd forgeio
   ```
3. **Build & run**

   ```bash
   cargo build
   cargo run --bin gateway_server
   ```
4. **Launch the design environment**

   ```bash
   cd frontend_design_env
   wasm-pack build
   ```

Refer to the [docs/](docs/) directory for detailed guides.

---

## Contributing

Your contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/foo`)
3. Commit your changes (`git commit -am 'Add foo'`)
4. Push to the branch (`git push origin feature/foo`)
5. Open a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for full details.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
