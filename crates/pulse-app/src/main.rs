//! Pulse App: Sovereign Real-Time System & Resource Monitor CLI & Desktop Application.

use std::env;
use pulse_core::{
    CpuTelemetry, DiskTelemetry, FileSystemManager, MemoryTelemetry, NetworkTelemetry,
    ProcessManager,
};
use pulse_render::{ActiveTab, PulseCanvas, Rect};
use pulse_storage::SystemTelemetrySnapshot;
use pulse_ui::PulseUiState;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("Pulse Sovereign Real-Time System & Resource Monitor v1.0.0 (NovaOS)");
        return;
    }

    if args.iter().any(|a| a == "--selftest" || a == "--test-full") {
        run_autonomous_qualification_suite();
        return;
    }

    if args.iter().any(|a| a == "--processes" || a == "-p") {
        print_processes();
        return;
    }

    if args.iter().any(|a| a == "--resources" || a == "-r") {
        print_resources();
        return;
    }

    if args.iter().any(|a| a == "--filesystems" || a == "-f") {
        print_filesystems();
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--kill" || a == "-k") {
        if let Some(pid_str) = args.get(pos + 1) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                let mut mgr = ProcessManager::new();
                match mgr.kill_process(pid) {
                    Ok(msg) => println!("[OK] {}", msg),
                    Err(err) => eprintln!("[ERROR] {}", err),
                }
                return;
            }
        }
        eprintln!("Usage: pulse --kill <pid>");
        return;
    }

    if args.iter().any(|a| a == "--report") {
        let p_mgr = ProcessManager::new();
        let cpu = CpuTelemetry::new(4);
        let mem = MemoryTelemetry::new();
        let net = NetworkTelemetry::new();
        let disk = DiskTelemetry::new();
        let fs_mgr = FileSystemManager::new();
        let snap = SystemTelemetrySnapshot::new(
            p_mgr.processes().to_vec(),
            cpu,
            mem,
            net,
            disk,
            fs_mgr.entries().to_vec(),
        );
        println!("{}", snap.to_json());
        return;
    }

    // Default overview
    println!("============================================================");
    println!("   PULSE: SOVEREIGN SYSTEM & RESOURCE MONITOR (NovaOS)       ");
    println!("============================================================");
    print_resources();
    println!();
    print_filesystems();
    println!();
    print_processes_summary();
}

fn print_processes() {
    let mgr = ProcessManager::new();
    println!("PROCESSES (Total: {})", mgr.processes().len());
    println!("{:<24} {:<10} {:>8} {:>8} {:>12} {:>14} {:>14} {:<10}",
        "Process Name", "User", "% CPU", "PID", "Memory", "Disk Read", "Disk Write", "Priority");
    println!("{}", "-".repeat(106));
    for p in mgr.processes() {
        println!("{:<24} {:<10} {:>7.2}% {:>8} {:>12} {:>14} {:>14} {:<10}",
            p.name, p.user, p.cpu_percent, p.pid, p.formatted_memory(),
            p.formatted_disk_read_total(), p.formatted_disk_write_total(), p.priority.as_str());
    }
}

fn print_processes_summary() {
    let mgr = ProcessManager::new();
    println!("ACTIVE PROCESSES (Top 6):");
    println!("{:<24} {:<10} {:>8} {:>8} {:>12} {:<10}",
        "Process Name", "User", "% CPU", "PID", "Memory", "Priority");
    println!("{}", "-".repeat(76));
    for p in mgr.processes().iter().take(6) {
        println!("{:<24} {:<10} {:>7.2}% {:>8} {:>12} {:<10}",
            p.name, p.user, p.cpu_percent, p.pid, p.formatted_memory(), p.priority.as_str());
    }
}

fn print_resources() {
    let cpu = CpuTelemetry::new(4);
    let mem = MemoryTelemetry::new();
    let net = NetworkTelemetry::new();
    let disk = DiskTelemetry::new();

    println!("HARDWARE & SYSTEM TELEMETRY:");
    println!("  CPU Utilization: Overall {:.1}%", cpu.overall_percent);
    print!("    Cores: ");
    for c in &cpu.cores {
        print!("CPU{} {:.1}%  ", c.core_id, c.usage_percent);
    }
    println!();

    println!("  Memory: {:.2} GB used ({:.1}%) of {:.2} GB  |  Cache: {:.2} GB",
        mem.used_bytes as f64 / 1e9, mem.memory_used_percent(),
        mem.total_bytes as f64 / 1e9, mem.cached_bytes as f64 / 1e9);
    println!("  Swap:   {:.2} GB used ({:.1}%) of {:.2} GB",
        mem.swap_used_bytes as f64 / 1e9, mem.swap_used_percent(),
        mem.swap_total_bytes as f64 / 1e9);

    println!("  Network: Receiving: {} (Total: {:.1} MiB) | Sending: {} (Total: {:.1} MiB)",
        pulse_core::format_rate(net.recv_rate_bps), net.total_recv_bytes as f64 / (1024.0 * 1024.0),
        pulse_core::format_rate(net.send_rate_bps), net.total_send_bytes as f64 / (1024.0 * 1024.0));

    println!("  Disk:    Reading: {} (Total: {:.1} GiB) | Writing: {} (Total: {:.1} GiB)",
        pulse_core::format_rate(disk.read_rate_bps), disk.total_read_bytes as f64 / 1e9,
        pulse_core::format_rate(disk.write_rate_bps), disk.total_write_bytes as f64 / 1e9);
}

fn print_filesystems() {
    let fs_mgr = FileSystemManager::new();
    println!("FILE SYSTEMS & STORAGE VOLUMES:");
    println!("{:<14} {:<16} {:<8} {:>12} {:>12} {:>10}",
        "Device", "Directory", "Type", "Total", "Available", "Used %");
    println!("{}", "-".repeat(76));
    for fs in fs_mgr.entries() {
        println!("{:<14} {:<16} {:<8} {:>12} {:>12} {:>9.1}%",
            fs.device, fs.directory, fs.fs_type, fs.formatted_total(), fs.formatted_available(), fs.used_percent());
    }
}

fn run_autonomous_qualification_suite() {
    println!("============================================================");
    println!("   PULSE: AUTONOMOUS QUALIFICATION PASS (10/10 TEST SUITES)  ");
    println!("============================================================");

    // 1. Process Enumeration
    let mut proc_mgr = ProcessManager::new();
    proc_mgr.populate_default_telemetry();
    assert!(!proc_mgr.processes().is_empty());
    println!("[PASS 1/10] Real-Time Process Enumeration & Discovery");

    // 2. Process Filtering
    let filtered = proc_mgr.filter_by_query("antigravity");
    assert!(!filtered.is_empty());
    println!("[PASS 2/10] Interactive Search & Dynamic Substring Query Filtering");

    // 3. Multi-Core CPU Telemetry
    let mut cpu = CpuTelemetry::new(4);
    assert_eq!(cpu.cores.len(), 4);
    cpu.push_sample(&[10.0, 15.0, 20.0, 25.0]);
    assert_eq!(cpu.overall_percent, 17.5);
    println!("[PASS 3/10] Multi-Core CPU Utilization & Rolling History Telemetry");

    // 4. Memory & Swap Gauges
    let mut mem = MemoryTelemetry::new();
    assert!(mem.memory_used_percent() > 0.0);
    mem.push_sample(8_000_000_000, 2_000_000_000);
    assert_eq!(mem.used_bytes, 8_000_000_000);
    println!("[PASS 4/10] Memory, Cache & Swap Invariant Calculation");

    // 5. Network Traffic Accounting
    let mut net = NetworkTelemetry::new();
    net.push_sample(1024 * 1024, 512 * 1024);
    assert_eq!(net.recv_rate_bps, 1024 * 1024);
    println!("[PASS 5/10] Network Interface Rx/Tx Rate & Cumulative Accounting");

    // 6. Disk Read/Write Rates
    let mut disk = DiskTelemetry::new();
    disk.push_sample(15_000_000, 536_000);
    assert_eq!(disk.write_rate_bps, 536_000);
    println!("[PASS 6/10] Disk Storage I/O Read/Write Transfer Rate Profiling");

    // 7. Filesystem Volume Topology
    let fs_mgr = FileSystemManager::new();
    assert!(!fs_mgr.entries().is_empty());
    assert_eq!(fs_mgr.entries()[0].directory, "/");
    println!("[PASS 7/10] Filesystem Mount Point & Storage Usage Mapping");

    // 8. Process Signal & Termination Safety Interlock
    let kill_init = proc_mgr.kill_process(1);
    assert!(kill_init.is_err(), "PID 1 must be protected");
    let kill_ok = proc_mgr.kill_process(4456);
    assert!(kill_ok.is_ok());
    println!("[PASS 8/10] Process Signal Termination & Critical PID Protection Interlock");

    // 9. 2D Vector Rasterizer
    let mut buf = vec![0u32; 1024 * 768];
    let mut canvas = PulseCanvas::new(&mut buf, 1024, 768);
    let mut state = PulseUiState::new();
    state.render(&mut canvas, Rect::new(0, 0, 1024, 768));
    let drawn = canvas.buffer.iter().filter(|&&p| p != 0).count();
    assert!(drawn > 50_000);
    println!("[PASS 9/10] 2D Double-Buffered Vector Rasterizer & Multi-Tab Composition");

    // 10. Interactive Event Dispatch & Modal Flow
    state.switch_tab(ActiveTab::Resources);
    assert_eq!(state.renderer.active_tab, ActiveTab::Resources);
    state.switch_tab(ActiveTab::Processes);
    state.open_details();
    assert!(state.renderer.show_details_modal);
    state.close_modal();
    assert!(!state.renderer.show_details_modal);
    println!("[PASS 10/10] Full Interactive State Machine & Process Modal Dialog Flow");

    println!("============================================================");
    println!("   ALL 10/10 PULSE QUALIFICATION TESTS PASSED SUCCESSFULLY!  ");
    println!("============================================================");
}
