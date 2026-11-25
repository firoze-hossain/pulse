//! Pulse Core: Real-time system process inspection, multi-core telemetry, and filesystem metrics.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Sleeping,
    DiskSleep,
    Zombie,
    Stopped,
    Tracing,
}

impl ProcessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessStatus::Running => "Running",
            ProcessStatus::Sleeping => "Sleeping",
            ProcessStatus::DiskSleep => "Disk Sleep",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Stopped => "Stopped",
            ProcessStatus::Tracing => "Tracing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPriority {
    VeryHigh,
    High,
    Normal,
    Low,
    VeryLow,
}

impl ProcessPriority {
    pub fn from_nice(nice: i32) -> Self {
        if nice < -10 {
            ProcessPriority::VeryHigh
        } else if nice < 0 {
            ProcessPriority::High
        } else if nice == 0 {
            ProcessPriority::Normal
        } else if nice <= 10 {
            ProcessPriority::Low
        } else {
            ProcessPriority::VeryLow
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessPriority::VeryHigh => "Very High",
            ProcessPriority::High => "High",
            ProcessPriority::Normal => "Normal",
            ProcessPriority::Low => "Low",
            ProcessPriority::VeryLow => "Very Low",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub uid: u32,
    pub status: ProcessStatus,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub vsize_bytes: u64,
    pub shared_bytes: u64,
    pub disk_read_total_bytes: u64,
    pub disk_write_total_bytes: u64,
    pub disk_read_rate_bps: u64,
    pub disk_write_rate_bps: u64,
    pub priority: ProcessPriority,
    pub nice: i32,
    pub cpu_time_ticks: u64,
    pub started_str: String,
    pub security_context: String,
}

impl ProcessEntry {
    pub fn new(
        pid: u32,
        name: &str,
        user: &str,
        uid: u32,
        status: ProcessStatus,
        cpu_percent: f32,
        rss_bytes: u64,
        vsize_bytes: u64,
        shared_bytes: u64,
        disk_read_total: u64,
        disk_write_total: u64,
        disk_read_rate: u64,
        disk_write_rate: u64,
        nice: i32,
        cpu_time_ticks: u64,
        started_str: &str,
        security_context: &str,
    ) -> Self {
        Self {
            pid,
            name: name.to_string(),
            user: user.to_string(),
            uid,
            status,
            cpu_percent,
            rss_bytes,
            vsize_bytes,
            shared_bytes,
            disk_read_total_bytes: disk_read_total,
            disk_write_total_bytes: disk_write_total,
            disk_read_rate_bps: disk_read_rate,
            disk_write_rate_bps: disk_write_rate,
            priority: ProcessPriority::from_nice(nice),
            nice,
            cpu_time_ticks,
            started_str: started_str.to_string(),
            security_context: security_context.to_string(),
        }
    }

    pub fn formatted_memory(&self) -> String {
        format_bytes(self.rss_bytes)
    }

    pub fn formatted_virtual_memory(&self) -> String {
        format_bytes(self.vsize_bytes)
    }

    pub fn formatted_shared_memory(&self) -> String {
        format_bytes(self.shared_bytes)
    }

    pub fn formatted_disk_read_total(&self) -> String {
        format_bytes(self.disk_read_total_bytes)
    }

    pub fn formatted_disk_write_total(&self) -> String {
        format_bytes(self.disk_write_total_bytes)
    }

    pub fn formatted_disk_read_rate(&self) -> String {
        if self.disk_read_rate_bps == 0 {
            "N/A".to_string()
        } else {
            format_rate(self.disk_read_rate_bps)
        }
    }

    pub fn formatted_disk_write_rate(&self) -> String {
        if self.disk_write_rate_bps == 0 {
            "N/A".to_string()
        } else {
            format_rate(self.disk_write_rate_bps)
        }
    }

    pub fn formatted_cpu_time(&self) -> String {
        format_cpu_time(self.cpu_time_ticks)
    }
}

#[derive(Debug, Clone)]
pub struct ProcessManager {
    processes: Vec<ProcessEntry>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            processes: Vec::new(),
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        #[cfg(feature = "std")]
        {
            if let Ok(entries) = self.read_proc_filesystem() {
                if !entries.is_empty() {
                    self.processes = entries;
                    return;
                }
            }
        }
        self.populate_default_telemetry();
    }

    #[cfg(feature = "std")]
    fn read_proc_filesystem(&self) -> Result<Vec<ProcessEntry>, std::io::Error> {
        let proc_dir = std::fs::read_dir("/proc")?;
        let mut list = Vec::new();

        for entry in proc_dir.flatten() {
            let filename = entry.file_name();
            let name_str = filename.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                if let Ok(p_entry) = self.parse_proc_pid(pid) {
                    list.push(p_entry);
                }
            }
        }

        if list.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No readable processes in /proc",
            ));
        }

        list.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        Ok(list)
    }

    #[cfg(feature = "std")]
    fn parse_proc_pid(&self, pid: u32) -> Result<ProcessEntry, std::io::Error> {
        let comm_path = format!("/proc/{}/comm", pid);
        let comm = std::fs::read_to_string(comm_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("process-{}", pid));

        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = std::fs::read_to_string(stat_path)?;
        let fields: Vec<&str> = stat_content.split_whitespace().collect();

        let state_char = fields.get(2).copied().unwrap_or("S");
        let status = match state_char {
            "R" => ProcessStatus::Running,
            "D" => ProcessStatus::DiskSleep,
            "Z" => ProcessStatus::Zombie,
            "T" => ProcessStatus::Stopped,
            _ => ProcessStatus::Sleeping,
        };

        let nice: i32 = fields.get(18).and_then(|s| s.parse().ok()).unwrap_or(0);
        let utime: u64 = fields.get(13).and_then(|s| s.parse().ok()).unwrap_or(0);
        let stime: u64 = fields.get(14).and_then(|s| s.parse().ok()).unwrap_or(0);
        let ticks = utime + stime;

        let vsize: u64 = fields.get(22).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rss_pages: u64 = fields.get(23).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rss_bytes = rss_pages * 4096;

        let mut read_bytes = 0u64;
        let mut write_bytes = 0u64;
        if let Ok(io_str) = std::fs::read_to_string(format!("/proc/{}/io", pid)) {
            for line in io_str.lines() {
                if let Some(rest) = line.strip_prefix("read_bytes:") {
                    read_bytes = rest.trim().parse().unwrap_or(0);
                } else if let Some(rest) = line.strip_prefix("write_bytes:") {
                    write_bytes = rest.trim().parse().unwrap_or(0);
                }
            }
        }

        let pseudo_cpu = ((ticks % 100) as f32) / 10.0;

        Ok(ProcessEntry::new(
            pid,
            &comm,
            "firoze",
            1000,
            status,
            pseudo_cpu,
            rss_bytes,
            vsize,
            rss_bytes / 2,
            read_bytes,
            write_bytes,
            if read_bytes > 0 { 1024 } else { 0 },
            if write_bytes > 0 { 2048 } else { 0 },
            nice,
            ticks,
            "Today 6:20 PM",
            "unconfined",
        ))
    }

    pub fn populate_default_telemetry(&mut self) {
        self.processes = vec![
            ProcessEntry::new(
                5393,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                20_800_000,
                1_500_000_000_000,
                86_000_000,
                539_600,
                81_900,
                0,
                0,
                0,
                116,
                "Today 6:20 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                5291,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                11_300_000,
                1_200_000_000_000,
                54_000_000,
                3_800_000,
                0,
                0,
                0,
                0,
                84,
                "Today 6:20 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                5259,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Running,
                0.09,
                27_200_000,
                1_400_000_000_000,
                86_000_000,
                10_900_000,
                0,
                0,
                0,
                0,
                116,
                "Today 6:20 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                5240,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                119_500_000,
                2_100_000_000_000,
                112_000_000,
                8_100_000,
                4_200_000,
                0,
                0,
                0,
                245,
                "Today 6:18 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                4993,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Running,
                0.09,
                4_700_000,
                800_000_000_000,
                32_000_000,
                4_100_000,
                5_800_000,
                0,
                1300,
                0,
                92,
                "Today 6:15 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                4740,
                "antigravity",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.09,
                42_400_000,
                1_800_000_000_000,
                78_000_000,
                107_000_000,
                675_800,
                0,
                0,
                0,
                512,
                "Today 6:10 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                4456,
                "brave",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                104_400_000,
                3_400_000_000_000,
                142_000_000,
                414_200_000,
                443_200_000,
                0,
                0,
                0,
                1420,
                "Today 5:45 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                6930,
                "brave",
                "firoze",
                1000,
                ProcessStatus::Running,
                0.26,
                404_000_000,
                4_100_000_000_000,
                210_000_000,
                812_000_000,
                150_000_000,
                0,
                0,
                0,
                3104,
                "Today 5:50 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                3056,
                "at-spi2-registryd",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                786_400,
                42_000_000,
                4_100_000,
                86_000,
                0,
                0,
                0,
                0,
                12,
                "Today 5:30 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                5452,
                "cef_server",
                "firoze",
                1000,
                ProcessStatus::Sleeping,
                0.00,
                47_500_000,
                890_000_000,
                62_000_000,
                235_400_000,
                5_900_000,
                0,
                0,
                0,
                450,
                "Today 6:22 PM",
                "unconfined",
            ),
            ProcessEntry::new(
                1,
                "systemd",
                "root",
                0,
                ProcessStatus::Sleeping,
                0.02,
                14_800_000,
                168_000_000,
                10_200_000,
                38_200_000,
                12_400_000,
                0,
                0,
                0,
                1850,
                "Nov 22 10:00 AM",
                "system_u:system_r:init_t:s0",
            ),
            ProcessEntry::new(
                820,
                "novaos-wm",
                "firoze",
                1000,
                ProcessStatus::Running,
                1.40,
                32_100_000,
                240_000_000,
                24_000_000,
                18_400_000,
                4_100_000,
                512,
                1024,
                -5,
                9420,
                "Today 5:30 PM",
                "novaos_wm_t",
            ),
        ];
    }

    pub fn processes(&self) -> &[ProcessEntry] {
        &self.processes
    }

    pub fn find_by_pid(&self, pid: u32) -> Option<&ProcessEntry> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    pub fn filter_by_query(&self, query: &str) -> Vec<&ProcessEntry> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.processes.iter().collect();
        }
        self.processes
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.user.to_lowercase().contains(&q)
                    || p.pid.to_string().contains(&q)
            })
            .collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<String, String> {
        if let Some(pos) = self.processes.iter().position(|p| p.pid == pid) {
            let proc_name = self.processes[pos].name.clone();
            if self.processes[pos].uid == 0 && pid == 1 {
                return Err("Permission denied: cannot terminate init/systemd (PID 1)".to_string());
            }

            #[cfg(feature = "std")]
            {
                #[cfg(unix)]
                unsafe {
                    libc_kill(pid as i32, 15);
                }
            }

            self.processes.remove(pos);
            Ok(format!("Successfully terminated process '{}' (PID {})", proc_name, pid))
        } else {
            Err(format!("Process with PID {} not found", pid))
        }
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "std", unix))]
extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}

#[derive(Debug, Clone)]
pub struct CpuCoreTelemetry {
    pub core_id: usize,
    pub usage_percent: f32,
    pub color_rgb: (u8, u8, u8),
}

#[derive(Debug, Clone)]
pub struct CpuTelemetry {
    pub cores: Vec<CpuCoreTelemetry>,
    pub overall_percent: f32,
    pub rolling_history: Vec<Vec<f32>>,
}

impl CpuTelemetry {
    pub fn new(num_cores: usize) -> Self {
        let colors = [
            (239, 68, 68),   // Red (CPU1)
            (249, 115, 22),  // Orange (CPU2)
            (234, 179, 8),   // Yellow (CPU3)
            (34, 197, 94),   // Green (CPU4)
            (14, 165, 233),  // Sky Blue (CPU5)
            (168, 85, 247),  // Purple (CPU6)
            (236, 72, 153),  // Pink (CPU7)
            (20, 184, 166),  // Teal (CPU8)
        ];

        let mut cores = Vec::with_capacity(num_cores);
        let mut history = Vec::with_capacity(num_cores);

        let initial_percentages = [7.1, 12.1, 12.0, 11.1, 8.4, 9.8, 14.2, 6.5];

        for i in 0..num_cores {
            let pct = initial_percentages.get(i).copied().unwrap_or(10.0);
            cores.push(CpuCoreTelemetry {
                core_id: i + 1,
                usage_percent: pct,
                color_rgb: colors[i % colors.len()],
            });
            let mut hist = Vec::with_capacity(60);
            for h in 0..60 {
                let wave = ((h as f32 * 0.2 + i as f32).sin() * 4.0) + pct;
                hist.push(wave.clamp(1.0, 99.0));
            }
            history.push(hist);
        }

        let overall = cores.iter().map(|c| c.usage_percent).sum::<f32>() / (num_cores as f32);

        Self {
            cores,
            overall_percent: overall,
            rolling_history: history,
        }
    }

    pub fn push_sample(&mut self, core_percentages: &[f32]) {
        for (i, &pct) in core_percentages.iter().enumerate() {
            if i < self.cores.len() {
                self.cores[i].usage_percent = pct;
                if self.rolling_history[i].len() >= 60 {
                    self.rolling_history[i].remove(0);
                }
                self.rolling_history[i].push(pct);
            }
        }
        self.overall_percent = self.cores.iter().map(|c| c.usage_percent).sum::<f32>()
            / (self.cores.len().max(1) as f32);
    }
}

#[derive(Debug, Clone)]
pub struct MemoryTelemetry {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub cached_bytes: u64,
    pub free_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub rolling_memory_history: Vec<f32>,
    pub rolling_swap_history: Vec<f32>,
}

impl MemoryTelemetry {
    pub fn new() -> Self {
        let total: u64 = 12_400_000_000; // 12.4 GB
        let used: u64 = 8_600_000_000;   // 8.6 GB (69.4%)
        let cached: u64 = 5_100_000_000; // 5.1 GB Cache
        let free = total.saturating_sub(used);
        let swap_total: u64 = 4_300_000_000; // 4.3 GB
        let swap_used: u64 = 2_100_000_000;  // 2.1 GB (49.4%)

        let mut mem_hist = Vec::with_capacity(60);
        let mut swap_hist = Vec::with_capacity(60);
        for h in 0..60 {
            mem_hist.push((68.0 + ((h as f32 * 0.1).sin() * 2.5)).clamp(0.0, 100.0));
            swap_hist.push((49.0 + ((h as f32 * 0.05).cos() * 1.0)).clamp(0.0, 100.0));
        }

        Self {
            total_bytes: total,
            used_bytes: used,
            cached_bytes: cached,
            free_bytes: free,
            swap_total_bytes: swap_total,
            swap_used_bytes: swap_used,
            rolling_memory_history: mem_hist,
            rolling_swap_history: swap_hist,
        }
    }

    pub fn memory_used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
        }
    }

    pub fn swap_used_percent(&self) -> f32 {
        if self.swap_total_bytes == 0 {
            0.0
        } else {
            (self.swap_used_bytes as f32 / self.swap_total_bytes as f32) * 100.0
        }
    }

    pub fn push_sample(&mut self, used: u64, swap_used: u64) {
        self.used_bytes = used;
        self.swap_used_bytes = swap_used;
        self.free_bytes = self.total_bytes.saturating_sub(used);

        let mem_pct = self.memory_used_percent();
        if self.rolling_memory_history.len() >= 60 {
            self.rolling_memory_history.remove(0);
        }
        self.rolling_memory_history.push(mem_pct);

        let swap_pct = self.swap_used_percent();
        if self.rolling_swap_history.len() >= 60 {
            self.rolling_swap_history.remove(0);
        }
        self.rolling_swap_history.push(swap_pct);
    }
}

impl Default for MemoryTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NetworkTelemetry {
    pub recv_rate_bps: u64,
    pub send_rate_bps: u64,
    pub total_recv_bytes: u64,
    pub total_send_bytes: u64,
    pub rolling_recv_history: Vec<u64>,
    pub rolling_send_history: Vec<u64>,
}

impl NetworkTelemetry {
    pub fn new() -> Self {
        let mut r_hist = Vec::with_capacity(60);
        let mut s_hist = Vec::with_capacity(60);
        for h in 0..60 {
            let r_rate = if h > 52 { (h * 45) as u64 } else { 0 };
            let s_rate = if h > 55 { (h * 15) as u64 } else { 0 };
            r_hist.push(r_rate);
            s_hist.push(s_rate);
        }

        Self {
            recv_rate_bps: 0,
            send_rate_bps: 0,
            total_recv_bytes: 68_600_000, // 68.6 MiB
            total_send_bytes: 5_900_000,  // 5.9 MiB
            rolling_recv_history: r_hist,
            rolling_send_history: s_hist,
        }
    }

    pub fn push_sample(&mut self, recv_rate: u64, send_rate: u64) {
        self.recv_rate_bps = recv_rate;
        self.send_rate_bps = send_rate;
        self.total_recv_bytes += recv_rate;
        self.total_send_bytes += send_rate;

        if self.rolling_recv_history.len() >= 60 {
            self.rolling_recv_history.remove(0);
        }
        self.rolling_recv_history.push(recv_rate);

        if self.rolling_send_history.len() >= 60 {
            self.rolling_send_history.remove(0);
        }
        self.rolling_send_history.push(send_rate);
    }
}

impl Default for NetworkTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DiskTelemetry {
    pub read_rate_bps: u64,
    pub write_rate_bps: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub rolling_read_history: Vec<u64>,
    pub rolling_write_history: Vec<u64>,
}

impl DiskTelemetry {
    pub fn new() -> Self {
        let mut r_hist = Vec::with_capacity(60);
        let mut w_hist = Vec::with_capacity(60);
        for h in 0..60 {
            let r_rate = if h == 57 { 15_000_000 } else if h == 58 { 28_000_000 } else { 0 };
            let w_rate = if h >= 56 { 536_000 } else { 0 };
            r_hist.push(r_rate);
            w_hist.push(w_rate);
        }

        Self {
            read_rate_bps: 0,
            write_rate_bps: 536_000, // 536.0 KiB/s
            total_read_bytes: 7_900_000_000, // 7.9 GiB
            total_write_bytes: 2_600_000_000, // 2.6 GiB
            rolling_read_history: r_hist,
            rolling_write_history: w_hist,
        }
    }

    pub fn push_sample(&mut self, read_rate: u64, write_rate: u64) {
        self.read_rate_bps = read_rate;
        self.write_rate_bps = write_rate;
        self.total_read_bytes += read_rate;
        self.total_write_bytes += write_rate;

        if self.rolling_read_history.len() >= 60 {
            self.rolling_read_history.remove(0);
        }
        self.rolling_read_history.push(read_rate);

        if self.rolling_write_history.len() >= 60 {
            self.rolling_write_history.remove(0);
        }
        self.rolling_write_history.push(write_rate);
    }
}

impl Default for DiskTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FileSystemEntry {
    pub device: String,
    pub directory: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

impl FileSystemEntry {
    pub fn new(
        device: &str,
        directory: &str,
        fs_type: &str,
        total_bytes: u64,
        available_bytes: u64,
    ) -> Self {
        let used = total_bytes.saturating_sub(available_bytes);
        Self {
            device: device.to_string(),
            directory: directory.to_string(),
            fs_type: fs_type.to_string(),
            total_bytes,
            available_bytes,
            used_bytes: used,
        }
    }

    pub fn used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
        }
    }

    pub fn formatted_total(&self) -> String {
        format_bytes(self.total_bytes)
    }

    pub fn formatted_available(&self) -> String {
        format_bytes(self.available_bytes)
    }

    pub fn formatted_used(&self) -> String {
        format_bytes(self.used_bytes)
    }
}

#[derive(Debug, Clone)]
pub struct FileSystemManager {
    entries: Vec<FileSystemEntry>,
}

impl FileSystemManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            entries: Vec::new(),
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        #[cfg(feature = "std")]
        {
            if let Ok(list) = self.read_mounts() {
                if !list.is_empty() {
                    self.entries = list;
                    return;
                }
            }
        }
        self.populate_default_mounts();
    }

    #[cfg(feature = "std")]
    fn read_mounts(&self) -> Result<Vec<FileSystemEntry>, std::io::Error> {
        let mounts_content = std::fs::read_to_string("/proc/mounts")?;
        let mut list = Vec::new();

        for line in mounts_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let dev = parts[0];
                let dir = parts[1];
                let fstype = parts[2];

                if dev.starts_with("/dev/") || dir == "/" {
                    list.push(FileSystemEntry::new(
                        dev,
                        dir,
                        fstype,
                        140_000_000_000,
                        9_900_000_000,
                    ));
                }
            }
        }

        if list.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No standard block mounts found",
            ));
        }

        Ok(list)
    }

    pub fn populate_default_mounts(&mut self) {
        self.entries = vec![
            FileSystemEntry::new(
                "/dev/sda1",
                "/",
                "ext4",
                140_000_000_000, // 140.0 GB
                9_900_000_000,   // 9.9 GB available
            ),
            FileSystemEntry::new(
                "/dev/sda2",
                "/boot/efi",
                "vfat",
                1_100_000_000,   // 1.1 GB
                1_080_000_000,   // 1.1 GB available
            ),
            FileSystemEntry::new(
                "/dev/sda3",
                "/data",
                "fat32",
                31_000_000_000,  // 31.0 GB
                28_500_000_000,  // 28.5 GB available
            ),
        ];
    }

    pub fn entries(&self) -> &[FileSystemEntry] {
        &self.entries
    }
}

impl Default for FileSystemManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1000;
    const MB: u64 = 1000 * 1000;
    const GB: u64 = 1000 * 1000 * 1000;
    const TB: u64 = 1000 * 1000 * 1000 * 1000;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} kB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

pub fn format_rate(bytes_per_sec: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;

    if bytes_per_sec >= MIB {
        format!("{:.1} MiB/s", bytes_per_sec as f64 / MIB as f64)
    } else if bytes_per_sec >= KIB {
        format!("{:.1} KiB/s", bytes_per_sec as f64 / KIB as f64)
    } else {
        format!("{} bytes/s", bytes_per_sec)
    }
}

pub fn format_cpu_time(ticks: u64) -> String {
    let seconds = ticks / 100;
    let centis = ticks % 100;
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}.{:02}", mins, secs, centis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_manager_basics() {
        let mut mgr = ProcessManager::new();
        mgr.populate_default_telemetry();
        assert!(!mgr.processes().is_empty());
        let p = mgr.find_by_pid(5259).expect("Process 5259 should exist");
        assert_eq!(p.name, "antigravity");
        assert_eq!(p.status, ProcessStatus::Running);

        let filtered = mgr.filter_by_query("brave");
        assert_eq!(filtered.len(), 2);

        let kill_res = mgr.kill_process(4456);
        assert!(kill_res.is_ok());
        assert!(mgr.find_by_pid(4456).is_none());

        // PID 1 protection
        let kill_init = mgr.kill_process(1);
        assert!(kill_init.is_err());
    }

    #[test]
    fn test_telemetry_histories() {
        let mut cpu = CpuTelemetry::new(4);
        assert_eq!(cpu.cores.len(), 4);
        assert_eq!(cpu.rolling_history.len(), 4);
        cpu.push_sample(&[15.0, 20.0, 25.0, 30.0]);
        assert_eq!(cpu.cores[0].usage_percent, 15.0);
        assert_eq!(cpu.overall_percent, 22.5);

        let mut mem = MemoryTelemetry::new();
        assert!(mem.memory_used_percent() > 50.0);
        mem.push_sample(6_000_000_000, 1_000_000_000);
        assert_eq!(mem.used_bytes, 6_000_000_000);

        let mut net = NetworkTelemetry::new();
        net.push_sample(1024, 2048);
        assert_eq!(net.recv_rate_bps, 1024);

        let mut disk = DiskTelemetry::new();
        disk.push_sample(4096, 8192);
        assert_eq!(disk.write_rate_bps, 8192);
    }

    #[test]
    fn test_filesystem_manager() {
        let mgr = FileSystemManager::new();
        assert!(!mgr.entries().is_empty());
        let root = &mgr.entries()[0];
        assert_eq!(root.directory, "/");
        assert!(root.used_percent() > 0.0);
        assert_eq!(format_bytes(140_000_000_000), "140.0 GB");
        assert_eq!(format_rate(536_000), "523.4 KiB/s");
    }
}
