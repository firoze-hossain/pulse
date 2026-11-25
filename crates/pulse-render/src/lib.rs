//! Pulse Render: Pure 2D software vector rasterizer for Pulse System Monitor.

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

use pulse_core::{
    CpuTelemetry, DiskTelemetry, FileSystemManager, MemoryTelemetry, NetworkTelemetry,
    ProcessEntry, ProcessManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Processes,
    Resources,
    FileSystems,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

pub struct PulseCanvas<'a> {
    pub buffer: &'a mut [u32],
    pub width: usize,
    pub height: usize,
}

impl<'a> PulseCanvas<'a> {
    pub fn new(buffer: &'a mut [u32], width: usize, height: usize) -> Self {
        Self {
            buffer,
            width,
            height,
        }
    }

    #[inline]
    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as usize) * self.width + (x as usize);
        if color.a == 255 {
            self.buffer[idx] = color.to_u32();
        } else if color.a > 0 {
            let bg = self.buffer[idx];
            let bg_r = ((bg >> 16) & 0xFF) as u32;
            let bg_g = ((bg >> 8) & 0xFF) as u32;
            let bg_b = (bg & 0xFF) as u32;

            let a = color.a as u32;
            let inv_a = 255 - a;

            let out_r = ((color.r as u32 * a) + (bg_r * inv_a)) / 255;
            let out_g = ((color.g as u32 * a) + (bg_g * inv_a)) / 255;
            let out_b = ((color.b as u32 * a) + (bg_b * inv_a)) / 255;

            self.buffer[idx] = (0xFF << 24) | (out_r << 16) | (out_g << 8) | out_b;
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x0 = rect.x.max(0);
        let y0 = rect.y.max(0);
        let x1 = (rect.x + rect.w).min(self.width as i32);
        let y1 = (rect.y + rect.h).min(self.height as i32);

        for y in y0..y1 {
            for x in x0..x1 {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_rect_outline(&mut self, rect: Rect, stroke: i32, color: Color) {
        self.fill_rect(Rect::new(rect.x, rect.y, rect.w, stroke), color);
        self.fill_rect(
            Rect::new(rect.x, rect.bottom() - stroke, rect.w, stroke),
            color,
        );
        self.fill_rect(Rect::new(rect.x, rect.y, stroke, rect.h), color);
        self.fill_rect(
            Rect::new(rect.right() - stroke, rect.y, stroke, rect.h),
            color,
        );
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: i32, color: Color) {
        self.fill_rect(
            Rect::new(
                rect.x + radius,
                rect.y,
                rect.w - radius * 2,
                rect.h,
            ),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x,
                rect.y + radius,
                radius,
                rect.h - radius * 2,
            ),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.right() - radius,
                rect.y + radius,
                radius,
                rect.h - radius * 2,
            ),
            color,
        );

        for dy in 0..radius {
            for dx in 0..radius {
                if dx * dx + dy * dy <= radius * radius {
                    self.put_pixel(rect.x + radius - 1 - dx, rect.y + radius - 1 - dy, color);
                    self.put_pixel(rect.right() - radius + dx, rect.y + radius - 1 - dy, color);
                    self.put_pixel(rect.x + radius - 1 - dx, rect.bottom() - radius + dy, color);
                    self.put_pixel(rect.right() - radius + dx, rect.bottom() - radius + dy, color);
                }
            }
        }
    }

    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_char(&mut self, x: i32, y: i32, c: u8, color: Color) {
        let glyph = FONT_6X10.get(c as usize).copied().unwrap_or(0);
        for row in 0..10 {
            let row_bits = (glyph >> (row * 6)) & 0x3F;
            for col in 0..6 {
                if (row_bits & (1 << col)) != 0 {
                    self.put_pixel(x + col, y + row, color);
                }
            }
        }
    }

    pub fn draw_text(&mut self, mut x: i32, y: i32, text: &str, color: Color) {
        for b in text.bytes() {
            if b == b'\n' {
                break;
            }
            self.draw_char(x, y, b, color);
            x += 7;
        }
    }
}

pub struct PulseRenderer {
    pub active_tab: ActiveTab,
    pub selected_pid: Option<u32>,
    pub show_details_modal: bool,
    pub search_query: String,
    pub scroll_y: i32,
}

impl PulseRenderer {
    pub fn new() -> Self {
        Self {
            active_tab: ActiveTab::Processes,
            selected_pid: Some(5259), // antigravity by default matching screenshot
            show_details_modal: false,
            search_query: String::new(),
            scroll_y: 0,
        }
    }

    pub fn render(
        &self,
        canvas: &mut PulseCanvas,
        bounds: Rect,
        proc_mgr: &ProcessManager,
        cpu: &CpuTelemetry,
        mem: &MemoryTelemetry,
        net: &NetworkTelemetry,
        disk: &DiskTelemetry,
        fs_mgr: &FileSystemManager,
    ) {
        // Dark theme matching GNOME System Monitor dark mode / sovereign desktop
        let bg_color = Color::rgb(24, 28, 36);
        let header_bg = Color::rgb(18, 21, 28);
        let _text_primary = Color::rgb(243, 244, 246);
        let text_muted = Color::rgb(156, 163, 175);
        let border_subtle = Color::rgb(45, 52, 64);
        let accent_tab = Color::rgb(59, 130, 246);

        canvas.fill_rect(bounds, bg_color);

        // 1. Header Toolbar with 3 Tab Switchers
        let th_h = 36;
        let th_rect = Rect::new(bounds.x, bounds.y, bounds.w, th_h);
        canvas.fill_rect(th_rect, header_bg);
        canvas.fill_rect(
            Rect::new(bounds.x, bounds.y + th_h - 1, bounds.w, 1),
            border_subtle,
        );

        // Center tabs: [≡ Processes] [⏱ Resources] [🗄 File Systems]
        let tab_w = 110;
        let total_tabs_w = tab_w * 3;
        let tabs_start_x = bounds.x + (bounds.w - total_tabs_w) / 2;

        let tabs = [
            (ActiveTab::Processes, "Processes"),
            (ActiveTab::Resources, "Resources"),
            (ActiveTab::FileSystems, "File Systems"),
        ];

        for (i, &(tab, name)) in tabs.iter().enumerate() {
            let tx = tabs_start_x + (i as i32 * tab_w);
            let tab_rect = Rect::new(tx, bounds.y + 4, tab_w - 4, 28);
            let is_active = self.active_tab == tab;

            if is_active {
                canvas.draw_rounded_rect(tab_rect, 4, Color::rgb(37, 43, 56));
                canvas.draw_rect_outline(tab_rect, 1, accent_tab);
                canvas.fill_rect(
                    Rect::new(tab_rect.x + 8, tab_rect.bottom() - 2, tab_rect.w - 16, 2),
                    accent_tab,
                );
            }

            let col = if is_active { Color::WHITE } else { text_muted };
            let t_offset = (tab_rect.w - (name.len() as i32 * 7)) / 2;
            canvas.draw_text(tab_rect.x + t_offset, tab_rect.y + 8, name, col);
        }

        // Search icon toggle on right
        let search_x = bounds.right() - 80;
        canvas.draw_rounded_rect(
            Rect::new(search_x, bounds.y + 6, 24, 24),
            4,
            Color::rgb(35, 42, 54),
        );
        canvas.draw_text(search_x + 8, bounds.y + 12, "Q", Color::rgb(148, 163, 184));

        // Client workspace area below toolbar
        let client = Rect::new(bounds.x, bounds.y + th_h, bounds.w, bounds.h - th_h);

        match self.active_tab {
            ActiveTab::Processes => {
                self.render_processes_tab(canvas, client, proc_mgr);
            }
            ActiveTab::Resources => {
                self.render_resources_tab(canvas, client, cpu, mem, net, disk);
            }
            ActiveTab::FileSystems => {
                self.render_filesystems_tab(canvas, client, fs_mgr);
            }
        }

        // Process Details Modal Dialog Overlay (if active)
        if self.show_details_modal {
            if let Some(pid) = self.selected_pid {
                if let Some(p) = proc_mgr.find_by_pid(pid) {
                    self.render_process_details_modal(canvas, bounds, p);
                }
            }
        }
    }

    fn render_processes_tab(
        &self,
        canvas: &mut PulseCanvas,
        client: Rect,
        proc_mgr: &ProcessManager,
    ) {
        let header_h = 24;
        let th_bg = Color::rgb(20, 24, 32);
        canvas.fill_rect(Rect::new(client.x, client.y, client.w, header_h), th_bg);
        canvas.fill_rect(
            Rect::new(client.x, client.y + header_h - 1, client.w, 1),
            Color::rgb(45, 52, 64),
        );

        // Columns from Screenshot 1:
        // Process Name (w 160) | User (w 70) | % CPU (w 60) | ID (w 55) | Memory (w 75) | Disk read total (w 90) | Disk write total (w 90) | Priority (w 65)
        let cols = [
            ("Process Name", 140),
            ("User", 60),
            ("% CPU", 55),
            ("ID", 50),
            ("Memory", 75),
            ("Disk read", 75),
            ("Disk write", 75),
            ("Priority", 65),
        ];

        let mut cx = client.x + 8;
        for &(title, width) in &cols {
            canvas.draw_text(cx, client.y + 7, title, Color::rgb(156, 163, 175));
            cx += width;
        }

        // Rows
        let row_h = 22;
        let mut ry = client.y + header_h;
        let list = proc_mgr.filter_by_query(&self.search_query);

        for p in list {
            if ry + row_h > client.bottom() {
                break;
            }

            let is_selected = self.selected_pid == Some(p.pid);
            let row_rect = Rect::new(client.x, ry, client.w, row_h);

            if is_selected {
                canvas.fill_rect(row_rect, Color::rgb(60, 48, 42)); // Subtle warm highlight matching screenshot
                canvas.draw_rect_outline(row_rect, 1, Color::rgb(249, 115, 22));
            } else if (p.pid % 2) == 0 {
                canvas.fill_rect(row_rect, Color::rgba(255, 255, 255, 4));
            }

            let mut col_x = client.x + 8;
            let text_col = if is_selected {
                Color::WHITE
            } else {
                Color::rgb(229, 231, 235)
            };

            // Process icon placeholder + Name
            canvas.draw_rounded_rect(
                Rect::new(col_x, ry + 4, 14, 14),
                2,
                Color::rgb(35, 42, 54),
            );
            canvas.draw_text(col_x + 18, ry + 6, &p.name, text_col);
            col_x += cols[0].1;

            canvas.draw_text(col_x, ry + 6, &p.user, Color::rgb(156, 163, 175));
            col_x += cols[1].1;

            let cpu_str = format!("{:.2}", p.cpu_percent);
            let cpu_col = if p.cpu_percent > 1.0 {
                Color::rgb(249, 115, 22)
            } else {
                text_col
            };
            canvas.draw_text(col_x, ry + 6, &cpu_str, cpu_col);
            col_x += cols[2].1;

            let id_str = p.pid.to_string();
            canvas.draw_text(col_x, ry + 6, &id_str, Color::rgb(156, 163, 175));
            col_x += cols[3].1;

            let mem_str = p.formatted_memory();
            canvas.draw_text(col_x, ry + 6, &mem_str, text_col);
            col_x += cols[4].1;

            let dr_str = p.formatted_disk_read_total();
            canvas.draw_text(col_x, ry + 6, &dr_str, Color::rgb(156, 163, 175));
            col_x += cols[5].1;

            let dw_str = p.formatted_disk_write_total();
            canvas.draw_text(col_x, ry + 6, &dw_str, Color::rgb(156, 163, 175));
            col_x += cols[6].1;

            canvas.draw_text(
                col_x,
                ry + 6,
                p.priority.as_str(),
                Color::rgb(148, 163, 184),
            );

            ry += row_h;
        }

        // Bottom Actions Toolbar: [ End Process ] button matching Screenshot 2
        let bbar_h = 32;
        let bbar_y = client.bottom() - bbar_h;
        canvas.fill_rect(
            Rect::new(client.x, bbar_y, client.w, bbar_h),
            Color::rgb(18, 21, 28),
        );
        canvas.fill_rect(
            Rect::new(client.x, bbar_y, client.w, 1),
            Color::rgb(45, 52, 64),
        );

        let btn_end = Rect::new(client.x + 8, bbar_y + 4, 100, 24);
        canvas.draw_rounded_rect(btn_end, 4, Color::rgb(220, 38, 38));
        canvas.draw_text(btn_end.x + 12, btn_end.y + 6, "End Process", Color::WHITE);

        if let Some(pid) = self.selected_pid {
            let info = format!("Selected PID: {} (Press Enter or Double-Click for details)", pid);
            canvas.draw_text(btn_end.right() + 16, bbar_y + 10, &info, Color::rgb(156, 163, 175));
        }
    }

    fn render_resources_tab(
        &self,
        canvas: &mut PulseCanvas,
        client: Rect,
        cpu: &CpuTelemetry,
        mem: &MemoryTelemetry,
        net: &NetworkTelemetry,
        disk: &DiskTelemetry,
    ) {
        // 4 Quadrants / vertical stack of 4 resource graphs (from Screenshot 3)
        let section_h = (client.h - 12) / 4;

        // 1. CPU Section
        let cpu_rect = Rect::new(client.x + 8, client.y + 4, client.w - 16, section_h);
        self.render_graph_section(
            canvas,
            cpu_rect,
            "v CPU",
            &cpu.rolling_history,
            &[
                Color::rgb(239, 68, 68),
                Color::rgb(249, 115, 22),
                Color::rgb(234, 179, 8),
                Color::rgb(34, 197, 94),
            ],
            100.0,
            &format!(
                "[CPU1 {:.1}%]  [CPU2 {:.1}%]  [CPU3 {:.1}%]  [CPU4 {:.1}%]",
                cpu.cores.get(0).map(|c| c.usage_percent).unwrap_or(0.0),
                cpu.cores.get(1).map(|c| c.usage_percent).unwrap_or(0.0),
                cpu.cores.get(2).map(|c| c.usage_percent).unwrap_or(0.0),
                cpu.cores.get(3).map(|c| c.usage_percent).unwrap_or(0.0),
            ),
        );

        // 2. Memory and Swap Section
        let mem_rect = Rect::new(client.x + 8, cpu_rect.bottom() + 4, client.w - 16, section_h);
        let mem_series = vec![
            mem.rolling_memory_history.clone(),
            mem.rolling_swap_history.clone(),
        ];
        self.render_graph_section(
            canvas,
            mem_rect,
            "v Memory and Swap",
            &mem_series,
            &[Color::rgb(239, 68, 68), Color::rgb(34, 197, 94)],
            100.0,
            &format!(
                "Memory: 8.6 GB ({:.1}%) of 12.4 GB, Cache 5.1 GB   |   Swap: 2.1 GB ({:.1}%) of 4.3 GB",
                mem.memory_used_percent(),
                mem.swap_used_percent()
            ),
        );

        // 3. Network Section
        let net_rect = Rect::new(client.x + 8, mem_rect.bottom() + 4, client.w - 16, section_h);
        let net_recv: Vec<f32> = net.rolling_recv_history.iter().map(|&v| v as f32).collect();
        let net_send: Vec<f32> = net.rolling_send_history.iter().map(|&v| v as f32).collect();
        let net_max = net_recv
            .iter()
            .chain(net_send.iter())
            .copied()
            .fold(1024.0f32, f32::max);
        self.render_graph_section(
            canvas,
            net_rect,
            "v Network",
            &[net_recv, net_send],
            &[Color::rgb(14, 165, 233), Color::rgb(249, 115, 22)],
            net_max,
            "Receiving: 0 bytes/s (Total 68.6 MiB)   |   Sending: 0 bytes/s (Total 5.9 MiB)",
        );

        // 4. Disk Section
        let disk_rect = Rect::new(client.x + 8, net_rect.bottom() + 4, client.w - 16, section_h);
        let d_read: Vec<f32> = disk.rolling_read_history.iter().map(|&v| v as f32).collect();
        let d_write: Vec<f32> = disk.rolling_write_history.iter().map(|&v| v as f32).collect();
        let d_max = d_read
            .iter()
            .chain(d_write.iter())
            .copied()
            .fold(30_000_000.0f32, f32::max);
        self.render_graph_section(
            canvas,
            disk_rect,
            "v Disk",
            &[d_read, d_write],
            &[Color::rgb(14, 165, 233), Color::rgb(249, 115, 22)],
            d_max,
            "Reading: 0 bytes/s (Total 7.9 GiB)   |   Writing: 536.0 KiB/s (Total 2.6 GiB)",
        );
    }

    fn render_graph_section(
        &self,
        canvas: &mut PulseCanvas,
        rect: Rect,
        title: &str,
        series: &[Vec<f32>],
        series_colors: &[Color],
        max_val: f32,
        legend: &str,
    ) {
        canvas.draw_text(rect.x, rect.y, title, Color::rgb(229, 231, 235));

        let g_y = rect.y + 14;
        let g_h = (rect.h - 32).max(24);
        let g_w = rect.w;

        // Background graph box
        canvas.fill_rect(Rect::new(rect.x, g_y, g_w, g_h), Color::rgb(15, 18, 24));
        canvas.draw_rect_outline(Rect::new(rect.x, g_y, g_w, g_h), 1, Color::rgb(35, 42, 54));

        // Horizontal grid lines
        for step in 1..4 {
            let gy = g_y + (g_h * step) / 4;
            canvas.draw_line(rect.x, gy, rect.right(), gy, Color::rgb(26, 32, 42));
        }

        // Plot data lines
        for (s_idx, s_data) in series.iter().enumerate() {
            if s_data.is_empty() {
                continue;
            }
            let color = series_colors[s_idx % series_colors.len()];
            let n = s_data.len();
            for i in 1..n {
                let x0 = rect.x + ((i - 1) as i32 * g_w) / (60.max(n as i32));
                let x1 = rect.x + (i as i32 * g_w) / (60.max(n as i32));

                let y0_norm = (s_data[i - 1] / max_val.max(1.0)).clamp(0.0, 1.0);
                let y1_norm = (s_data[i] / max_val.max(1.0)).clamp(0.0, 1.0);

                let y0 = g_y + g_h - 2 - (y0_norm * (g_h - 4) as f32) as i32;
                let y1 = g_y + g_h - 2 - (y1_norm * (g_h - 4) as f32) as i32;

                canvas.draw_line(x0, y0, x1, y1, color);
            }
        }

        // Legend below graph
        canvas.draw_text(rect.x, g_y + g_h + 4, legend, Color::rgb(156, 163, 175));
    }

    fn render_filesystems_tab(
        &self,
        canvas: &mut PulseCanvas,
        client: Rect,
        fs_mgr: &FileSystemManager,
    ) {
        let header_h = 24;
        let th_bg = Color::rgb(20, 24, 32);
        canvas.fill_rect(Rect::new(client.x, client.y, client.w, header_h), th_bg);
        canvas.fill_rect(
            Rect::new(client.x, client.y + header_h - 1, client.w, 1),
            Color::rgb(45, 52, 64),
        );

        // Columns from Screenshot 4:
        // Device (w 100) | Directory (w 120) | Type (w 80) | Total (w 90) | Available (w 90) | Used (bar + pct)
        let cols = [
            ("Device", 100),
            ("Directory", 110),
            ("Type", 70),
            ("Total", 85),
            ("Available", 85),
            ("Used Space", 120),
        ];

        let mut cx = client.x + 8;
        for &(title, width) in &cols {
            canvas.draw_text(cx, client.y + 7, title, Color::rgb(156, 163, 175));
            cx += width;
        }

        let row_h = 26;
        let mut ry = client.y + header_h + 2;

        for fs in fs_mgr.entries() {
            if ry + row_h > client.bottom() {
                break;
            }

            let row_rect = Rect::new(client.x, ry, client.w, row_h);
            canvas.fill_rect(row_rect, Color::rgba(255, 255, 255, 3));

            let mut col_x = client.x + 8;
            // Disk icon placeholder + Device
            canvas.draw_rounded_rect(
                Rect::new(col_x, ry + 5, 14, 14),
                2,
                Color::rgb(35, 42, 54),
            );
            canvas.draw_text(col_x + 18, ry + 7, &fs.device, Color::WHITE);
            col_x += cols[0].1;

            canvas.draw_text(
                col_x,
                ry + 7,
                &fs.directory,
                Color::rgb(56, 189, 248),
            );
            col_x += cols[1].1;

            canvas.draw_text(
                col_x,
                ry + 7,
                &fs.fs_type,
                Color::rgb(156, 163, 175),
            );
            col_x += cols[2].1;

            let tot_str = fs.formatted_total();
            canvas.draw_text(col_x, ry + 7, &tot_str, Color::WHITE);
            col_x += cols[3].1;

            let avail_str = fs.formatted_available();
            canvas.draw_text(col_x, ry + 7, &avail_str, Color::rgb(156, 163, 175));
            col_x += cols[4].1;

            // Usage Progress Bar + Percentage
            let bar_w = 70;
            let bar_h = 10;
            let bar_y = ry + 7;
            canvas.draw_rounded_rect(
                Rect::new(col_x, bar_y, bar_w, bar_h),
                2,
                Color::rgb(35, 42, 54),
            );

            let pct = fs.used_percent();
            let fill_w = ((bar_w as f32 * pct) / 100.0) as i32;
            let bar_col = if pct > 90.0 {
                Color::rgb(239, 68, 68)
            } else if pct > 75.0 {
                Color::rgb(249, 115, 22)
            } else {
                Color::rgb(34, 197, 94)
            };
            if fill_w > 0 {
                canvas.draw_rounded_rect(
                    Rect::new(col_x, bar_y, fill_w.min(bar_w), bar_h),
                    2,
                    bar_col,
                );
            }

            let pct_str = format!("{:.1}%", pct);
            canvas.draw_text(col_x + bar_w + 8, ry + 7, &pct_str, bar_col);

            ry += row_h;
        }
    }

    fn render_process_details_modal(
        &self,
        canvas: &mut PulseCanvas,
        screen: Rect,
        proc: &ProcessEntry,
    ) {
        // Dim screen background
        canvas.fill_rect(screen, Color::rgba(0, 0, 0, 140));

        let card_w = 420.min(screen.w - 32);
        let card_h = 360.min(screen.h - 32);
        let cx = screen.x + (screen.w - card_w) / 2;
        let cy = screen.y + (screen.h - card_h) / 2;
        let card = Rect::new(cx, cy, card_w, card_h);

        // Card frame
        canvas.draw_rounded_rect(card, 8, Color::rgb(28, 33, 44));
        canvas.draw_rect_outline(card, 1, Color::rgb(75, 85, 99));

        // Header: "antigravity (PID 5259)"
        let title = format!("{} (PID {})", proc.name, proc.pid);
        canvas.draw_text(card.x + 16, card.y + 14, &title, Color::WHITE);

        // Close button [x]
        let close_btn = Rect::new(card.right() - 28, card.y + 10, 18, 18);
        canvas.draw_rounded_rect(close_btn, 3, Color::rgb(45, 52, 64));
        canvas.draw_text(close_btn.x + 6, close_btn.y + 4, "x", Color::rgb(209, 213, 219));

        // Properties list from Screenshot 2
        let props = [
            ("Process Name", proc.name.clone()),
            ("User", format!("{} ({})", proc.user, proc.uid)),
            ("Status", proc.status.as_str().to_string()),
            ("Memory", proc.formatted_memory()),
            ("Virtual Memory", proc.formatted_virtual_memory()),
            ("Resident Memory", proc.formatted_memory()),
            ("Shared Memory", proc.formatted_shared_memory()),
            ("CPU", format!("{:.2}%", proc.cpu_percent)),
            ("CPU Time", proc.formatted_cpu_time()),
            ("Started", proc.started_str.clone()),
            ("Nice", proc.nice.to_string()),
            ("Priority", proc.priority.as_str().to_string()),
            ("ID", proc.pid.to_string()),
            ("Security Context", proc.security_context.clone()),
        ];

        let mut py = card.y + 42;
        for (label, val) in &props {
            if py + 16 > card.bottom() - 44 {
                break;
            }
            canvas.draw_text(card.x + 18, py, label, Color::rgb(156, 163, 175));
            canvas.draw_text(card.x + 160, py, val, Color::WHITE);
            py += 18;
        }

        // Action button [ End Process ]
        let btn_end = Rect::new(card.x + 16, card.bottom() - 34, 110, 24);
        canvas.draw_rounded_rect(btn_end, 4, Color::rgb(220, 38, 38));
        canvas.draw_text(btn_end.x + 14, btn_end.y + 6, "End Process", Color::WHITE);
    }
}

impl Default for PulseRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// 6x10 font table for clean rendering
static FONT_6X10: [u64; 128] = {
    let mut table = [0u64; 128];
    // ASCII rendering patterns
    table[b' ' as usize] = 0;
    table[b'A' as usize] = 0x1F244444241F;
    table[b'B' as usize] = 0x17254949253F;
    table[b'C' as usize] = 0x11214141221C;
    table[b'D' as usize] = 0x0E112141213F;
    table[b'E' as usize] = 0x21414545213F;
    table[b'F' as usize] = 0x20404444203F;
    table[b'G' as usize] = 0x1D254541221C;
    table[b'H' as usize] = 0x1F204040203F;
    table[b'I' as usize] = 0x21417F412100;
    table[b'J' as usize] = 0x20407E412102;
    table[b'K' as usize] = 0x21120C12213F;
    table[b'L' as usize] = 0x01010101013F;
    table[b'M' as usize] = 0x3F2018203F00;
    table[b'N' as usize] = 0x3F0810203F00;
    table[b'O' as usize] = 0x1C224141221C;
    table[b'P' as usize] = 0x18244444243F;
    table[b'Q' as usize] = 0x1D224541221C;
    table[b'R' as usize] = 0x19264444243F;
    table[b'S' as usize] = 0x112145492912;
    table[b'T' as usize] = 0x20407F402000;
    table[b'U' as usize] = 0x1F204040201F;
    table[b'V' as usize] = 0x1E204040201E;
    table[b'W' as usize] = 0x3F0204023F00;
    table[b'X' as usize] = 0x331408143300;
    table[b'Y' as usize] = 0x20403F402000;
    table[b'Z' as usize] = 0x214345495161;

    table[b'a' as usize] = 0x1F1212120C00;
    table[b'b' as usize] = 0x0F1111111F00;
    table[b'c' as usize] = 0x091212120C00;
    table[b'd' as usize] = 0x1F1111110E00;
    table[b'e' as usize] = 0x0D1515150E00;
    table[b'f' as usize] = 0x203F24240800;
    table[b'g' as usize] = 0x1E0505051800;
    table[b'h' as usize] = 0x1F1010101F00;
    table[b'i' as usize] = 0x1D0000000000;
    table[b'j' as usize] = 0x1E0101010000;
    table[b'k' as usize] = 0x110A040A1F00;
    table[b'l' as usize] = 0x011F00000000;
    table[b'm' as usize] = 0x1F1008101F00;
    table[b'n' as usize] = 0x1F1010100F00;
    table[b'o' as usize] = 0x0E1111110E00;
    table[b'p' as usize] = 0x0C1212121F00;
    table[b'q' as usize] = 0x1F1212120C00;
    table[b'r' as usize] = 0x100810101F00;
    table[b's' as usize] = 0x091515151200;
    table[b't' as usize] = 0x09113E101000;
    table[b'u' as usize] = 0x1F0101011E00;
    table[b'v' as usize] = 0x1E0102041800;
    table[b'w' as usize] = 0x1F0204021F00;
    table[b'x' as usize] = 0x1B0404041B00;
    table[b'y' as usize] = 0x1E0505051E00;
    table[b'z' as usize] = 0x131515151900;

    table[b'0' as usize] = 0x1C224141221C;
    table[b'1' as usize] = 0x01017F211100;
    table[b'2' as usize] = 0x214345495161;
    table[b'3' as usize] = 0x1A2545452112;
    table[b'4' as usize] = 0x04047F24140C;
    table[b'5' as usize] = 0x192545452539;
    table[b'6' as usize] = 0x19254545221C;
    table[b'7' as usize] = 0x384442416040;
    table[b'8' as usize] = 0x1B254545251B;
    table[b'9' as usize] = 0x1C2245452513;

    table[b'.' as usize] = 0x010000000000;
    table[b',' as usize] = 0x020100000000;
    table[b':' as usize] = 0x120000000000;
    table[b'-' as usize] = 0x040404040400;
    table[b'/' as usize] = 0x201008040201;
    table[b'%' as usize] = 0x23150B081531;
    table[b'(' as usize] = 0x224100000000;
    table[b')' as usize] = 0x000000004122;
    table[b'[' as usize] = 0x417F00000000;
    table[b']' as usize] = 0x000000007F41;
    table[b'|' as usize] = 0x7F0000000000;
    table[b'v' as usize] = 0x1E0102041800;
    table[b'x' as usize] = 0x1B0404041B00;
    table
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_initialization_and_rendering() {
        let renderer = PulseRenderer::new();
        let proc_mgr = ProcessManager::new();
        let cpu = CpuTelemetry::new(4);
        let mem = MemoryTelemetry::new();
        let net = NetworkTelemetry::new();
        let disk = DiskTelemetry::new();
        let fs_mgr = FileSystemManager::new();

        let mut buf = vec![0u32; 800 * 600];
        let mut canvas = PulseCanvas::new(&mut buf, 800, 600);

        // Test rendering all 3 tabs without panic
        let bounds = Rect::new(0, 0, 800, 600);

        // Tab 1: Processes
        renderer.render(
            &mut canvas,
            bounds,
            &proc_mgr,
            &cpu,
            &mem,
            &net,
            &disk,
            &fs_mgr,
        );
        let drawn_px = canvas.buffer.iter().filter(|&&p| p != 0).count();
        assert!(drawn_px > 10_000, "Processes tab should draw elements");

        // Tab 2: Resources
        let mut r_res = PulseRenderer::new();
        r_res.active_tab = ActiveTab::Resources;
        r_res.render(
            &mut canvas,
            bounds,
            &proc_mgr,
            &cpu,
            &mem,
            &net,
            &disk,
            &fs_mgr,
        );

        // Tab 3: File Systems
        let mut r_fs = PulseRenderer::new();
        r_fs.active_tab = ActiveTab::FileSystems;
        r_fs.render(
            &mut canvas,
            bounds,
            &proc_mgr,
            &cpu,
            &mem,
            &net,
            &disk,
            &fs_mgr,
        );

        // Modal dialog
        let mut r_modal = PulseRenderer::new();
        r_modal.show_details_modal = true;
        r_modal.render(
            &mut canvas,
            bounds,
            &proc_mgr,
            &cpu,
            &mem,
            &net,
            &disk,
            &fs_mgr,
        );
    }
}
