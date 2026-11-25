# Pulse: Sovereign Real-Time System & Resource Monitor

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Pure Rust](https://img.shields.io/badge/Language-Pure%20Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-NovaOS%20%7C%20Linux%20%7C%20macOS%20%7C%20Windows-green.svg)]()

**Pulse** is the sovereign real-time process manager, multi-core telemetry profiler, and filesystem resource monitor designed for **NovaOS**, synthesizing the complete dynamic capabilities and user interface of **Ubuntu System Monitor** (`gnome-system-monitor`).

---

## Capabilities

1. **Process Management (`[≡ Processes]`)**:
   - Live process discovery across PIDs, user ownership, memory usage, disk read/write bandwidth, and priority levels.
   - Interactive search filtering by process name, user, or PID.
   - Process Details modal inspector card displaying virtual, resident, shared memory, nice levels, accumulated CPU time, and security context.
   - Protected signal termination (`[ End Process ]`) with safety interlocks preventing termination of PID 1 (`init` / `systemd`).

2. **Real-Time Telemetry & Resource Gauging (`[⏱ Resources]`)**:
   - **Multi-Core CPU History**: Dynamic line graph plotting individual per-core usage with 60-second rolling sample history and color-coded core legends.
   - **Memory & Swap Gauges**: Dual live ring buffers charting physical memory and swap allocation alongside circular percentage meters.
   - **Network Traffic Accounting**: Live line graphs tracking receiving (downlink) and sending (uplink) bandwidth rates and cumulative transfer accounting.
   - **Disk Storage I/O Profiling**: Live line graphs charting read and write throughput and total data moved.

3. **Storage Volume Topology (`[🗄 File Systems]`)**:
   - Real-time enumeration of mounted storage devices (`/dev/sda1`, `/dev/sda2`, etc.).
   - Visual proportional colored usage bars and available capacity calculation.

4. **Sovereign Pure-Rust Architecture**:
   - Zero external C/C++ or GPU runtime dependencies.
   - 2D software double-buffered vector rasterizer for instant rendering on bare-metal framebuffers.
   - Cross-platform CLI with automated qualification suites.

---

## Workspace Layout

```
pulse/
├── crates/
│   ├── pulse-core/      # Process manager, multi-core telemetry, filesystem metrics
│   ├── pulse-storage/   # JSON reporting, procfs serialization, audit logging
│   ├── pulse-render/    # 2D software double-buffered vector rasterizer
│   ├── pulse-ui/        # Interactive event dispatch, tabs, and state machine
│   └── pulse-app/       # Standalone cross-platform CLI application
└── novaos/              # Bare-metal i686 ring-3 entry point, FFI, and build scripts
```

---

## Building and Running

```bash
# Run all workspace tests
cargo test

# Run the 10/10 autonomous qualification suite
cargo run -p pulse-app -- --test-full

# Inspect active processes
cargo run -p pulse-app -- --processes

# View live system hardware telemetry
cargo run -p pulse-app -- --resources

# View storage filesystems
cargo run -p pulse-app -- --filesystems

# Generate JSON telemetry report
cargo run -p pulse-app -- --report
```

---

## License

Dual-licensed under MIT or Apache 2.0. Developed by the NovaOS Architecture Team.
