# ForgeIO / OpenForge

> An open-source, Rust-based SCADA & Machine Interface platform

ForgeIO (also known as OpenForge) leverages Rust’s performance, safety, and concurrency to deliver a modern, high-performance solution for industrial automation. It's designed for demanding environments requiring scalability to millions of tags and robust data handling, featuring a comprehensive visual design environment and flexible edge deployment capabilities.

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

* **Collect** data from industrial devices via a highly extensible driver system (initially supporting OPC UA, Modbus, MQTT, Siemens S7, and more)
* **Manage** real-time tags and store historical data
* **Script** custom logic in Python (via PyO3) or Rust
* **Visualize** and **control** processes through a web-based drag-and-drop interface
* **Deploy** at the edge or in centralized gateways
* **Extend** and **collaborate** via open-source development

---

## Key Features

* **High-Performance & Scalable Tag Engine**: Designed for millions of real-time tags with efficient updates and low latency.
* **Data Acquisition Layer**: Extensible architecture supporting OPC UA, Modbus TCP/RTU, MQTT, Siemens S7, with the ability for community contributions.
* **Tag System & Historian**: Thread-safe in-memory tags, seamless time-series storage
* **Embedded Scripting**: Python 3+ integration for event handlers and calculations
* **Real-Time Events & Alarms**: Configurable triggers, notifications, and workflows
* **Visual Design Environment**: Intuitive drag-and-drop editor, script console, and project manager
* **Edge & Machine Interface**: Local buffering, protocol translation, and responsive Rust/WASM UIs
* **Version Control**: Git-native project storage and history

---

## Architecture

### Gateway Server / Edge Runtime

The core runtime can operate as a centralized Gateway or be deployed directly at the Edge (e.g., as a Machine Interface/HMI backend). Both modes share the same high-performance foundation but can be configured for different scales and resource constraints.

* **Data Acquisition**: Pluggable drivers for various protocols (OPC UA, Modbus, MQTT, Siemens S7, etc.). Built for high throughput and concurrent connections. Edge deployments prioritize low-latency direct device communication.
* **Tag Management**: Fast, concurrent tag reads/writes. Scalable engine designed to handle millions of tags efficiently.
* **Scripting Engine**: Embedded Python via PyO3 for flexible logic.
* **Event Processing**: Alarm management and notification services.
* **API Layer**: WebSockets, REST, gRPC endpoints for integration and UI communication.
* **Edge-Specific Optimizations**: Includes local data buffering for resilience, efficient data flow minimizing overhead, and optimized task scheduling for responsive HMI interactions.

### Edge & Machine Interface Capabilities (Leveraging Gateway Runtime)

When deployed at the edge, the runtime leverages the core components for:

* **Protocol Gateways**: Utilizing the standard, high-performance drivers for direct PLC communication.
* **Visualization UI Backend**: Providing real-time data via WebSockets or other efficient mechanisms to the Rust/WASM frontend.
* **Local Historian**: Temporary storage (potentially using lightweight databases or in-memory buffering) to support offline operation and ensure data integrity.

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

* **Frameworks**: Axum, Tokio (chosen for high-concurrency and performance)
* **Libraries**: tokio-modbus, opcua, rumqttc, potentially dedicated S7 libraries
* **Scripting**: PyO3
* **Storage**: SQLx, TimescaleDB
* **Realtime**: tokio-tungstenite (WebSockets)

### Frontend (Rust/WASM)

* **UI**: Leptos or Yew (chosen for performance and WASM integration)
* **Compilation**: WebAssembly for high performance
* **Components**: Custom drag-and-drop toolkit, optimized for high-frequency data updates

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

5. **Run the Admin Dashboard (React/TypeScript)**

   ```bash
   cd webui
   npm install
   npm run dev

   # Build for production to serve through the gateway
   npm run build
   ```

The gateway API now requires HTTP Basic auth with the default credentials
`admin`/`admin`.


Refer to the [docs/](docs/) directory for detailed guides.

### Dummy OPC UA Server for Testing

If you need an OPC UA endpoint for local development, a simple Python script is
included in `examples/dummy_opcua_server.py`. The script creates a server with a
few variables that change value every second.

1. Install the required package:

   ```bash
   pip install asyncua
   ```

2. Start the server:

   ```bash
   python examples/dummy_opcua_server.py
   ```

The server listens on `opc.tcp://localhost:4840/freeopcua/server/` and provides
`Temperature`, `Pressure`, and `Counter` nodes for testing reads and
subscriptions.

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
