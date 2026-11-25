//! Pulse Storage: Serialization and telemetry reporting for Pulse System Monitor.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use pulse_core::{
    CpuTelemetry, DiskTelemetry, FileSystemEntry, MemoryTelemetry, NetworkTelemetry, ProcessEntry,
};

#[derive(Debug, Clone)]
pub struct SystemTelemetrySnapshot {
    pub timestamp_epoch_secs: u64,
    pub processes: Vec<ProcessEntry>,
    pub cpu: CpuTelemetry,
    pub memory: MemoryTelemetry,
    pub network: NetworkTelemetry,
    pub disk: DiskTelemetry,
    pub filesystems: Vec<FileSystemEntry>,
}

impl SystemTelemetrySnapshot {
    pub fn new(
        processes: Vec<ProcessEntry>,
        cpu: CpuTelemetry,
        memory: MemoryTelemetry,
        network: NetworkTelemetry,
        disk: DiskTelemetry,
        filesystems: Vec<FileSystemEntry>,
    ) -> Self {
        Self {
            timestamp_epoch_secs: 1795493000,
            processes,
            cpu,
            memory,
            network,
            disk,
            filesystems,
        }
    }

    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str("  \"app\": \"Pulse System Monitor\",\n");
        json.push_str("  \"version\": \"1.0.0\",\n");
        json.push_str(&format!("  \"timestamp\": {},\n", self.timestamp_epoch_secs));

        // CPU
        json.push_str("  \"cpu\": {\n");
        json.push_str(&format!("    \"overall_percent\": {:.2},\n", self.cpu.overall_percent));
        json.push_str("    \"cores\": [\n");
        for (i, c) in self.cpu.cores.iter().enumerate() {
            let comma = if i + 1 == self.cpu.cores.len() { "" } else { "," };
            json.push_str(&format!(
                "      {{\"core\": {}, \"usage_percent\": {:.2}}}{}\n",
                c.core_id, c.usage_percent, comma
            ));
        }
        json.push_str("    ]\n");
        json.push_str("  },\n");

        // Memory
        json.push_str("  \"memory\": {\n");
        json.push_str(&format!("    \"total_bytes\": {},\n", self.memory.total_bytes));
        json.push_str(&format!("    \"used_bytes\": {},\n", self.memory.used_bytes));
        json.push_str(&format!("    \"cached_bytes\": {},\n", self.memory.cached_bytes));
        json.push_str(&format!("    \"used_percent\": {:.2},\n", self.memory.memory_used_percent()));
        json.push_str(&format!("    \"swap_total_bytes\": {},\n", self.memory.swap_total_bytes));
        json.push_str(&format!("    \"swap_used_bytes\": {},\n", self.memory.swap_used_bytes));
        json.push_str(&format!("    \"swap_used_percent\": {:.2}\n", self.memory.swap_used_percent()));
        json.push_str("  },\n");

        // Network
        json.push_str("  \"network\": {\n");
        json.push_str(&format!("    \"recv_rate_bps\": {},\n", self.network.recv_rate_bps));
        json.push_str(&format!("    \"send_rate_bps\": {},\n", self.network.send_rate_bps));
        json.push_str(&format!("    \"total_recv_bytes\": {},\n", self.network.total_recv_bytes));
        json.push_str(&format!("    \"total_send_bytes\": {}\n", self.network.total_send_bytes));
        json.push_str("  },\n");

        // Disk
        json.push_str("  \"disk\": {\n");
        json.push_str(&format!("    \"read_rate_bps\": {},\n", self.disk.read_rate_bps));
        json.push_str(&format!("    \"write_rate_bps\": {},\n", self.disk.write_rate_bps));
        json.push_str(&format!("    \"total_read_bytes\": {},\n", self.disk.total_read_bytes));
        json.push_str(&format!("    \"total_write_bytes\": {}\n", self.disk.total_write_bytes));
        json.push_str("  },\n");

        // Filesystems
        json.push_str("  \"filesystems\": [\n");
        for (i, fs) in self.filesystems.iter().enumerate() {
            let comma = if i + 1 == self.filesystems.len() { "" } else { "," };
            json.push_str(&format!(
                "    {{\"device\": \"{}\", \"mount\": \"{}\", \"type\": \"{}\", \"total\": {}, \"available\": {}, \"used_percent\": {:.2}}}{}\n",
                fs.device, fs.directory, fs.fs_type, fs.total_bytes, fs.available_bytes, fs.used_percent(), comma
            ));
        }
        json.push_str("  ],\n");

        // Processes count
        json.push_str(&format!("  \"process_count\": {}\n", self.processes.len()));
        json.push_str("}\n");

        json
    }

    pub fn to_proc_stat(&self) -> String {
        let mut out = String::new();
        out.push_str("cpu  1420 310 940 85200 450 120 40 0 0 0\n");
        for c in &self.cpu.cores {
            let user = (c.usage_percent * 10.0) as u64;
            let idle = ((100.0 - c.usage_percent) * 10.0) as u64;
            out.push_str(&format!("cpu{} {} 0 20 {} 10 0 0 0 0 0\n", c.core_id - 1, user, idle));
        }
        out.push_str("intr 1042938\n");
        out.push_str("ctxt 2894102\n");
        out.push_str("btime 1795400000\n");
        out.push_str(&format!("processes {}\n", self.processes.len()));
        out
    }

    pub fn to_proc_meminfo(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("MemTotal:        {} kB\n", self.memory.total_bytes / 1024));
        out.push_str(&format!("MemFree:         {} kB\n", self.memory.free_bytes / 1024));
        out.push_str(&format!("MemAvailable:    {} kB\n", (self.memory.free_bytes + self.memory.cached_bytes) / 1024));
        out.push_str(&format!("Cached:          {} kB\n", self.memory.cached_bytes / 1024));
        out.push_str(&format!("SwapTotal:       {} kB\n", self.memory.swap_total_bytes / 1024));
        out.push_str(&format!("SwapFree:        {} kB\n", (self.memory.swap_total_bytes - self.memory.swap_used_bytes) / 1024));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_core::{FileSystemManager, ProcessManager};

    #[test]
    fn test_storage_json_serialization() {
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

        let json = snap.to_json();
        assert!(json.contains("\"Pulse System Monitor\""));
        assert!(json.contains("\"overall_percent\""));
        assert!(json.contains("\"MemTotal:\"") == false);

        let stat = snap.to_proc_stat();
        assert!(stat.contains("cpu0"));

        let meminfo = snap.to_proc_meminfo();
        assert!(meminfo.contains("MemTotal:"));
    }
}
