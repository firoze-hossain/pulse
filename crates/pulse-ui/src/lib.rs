//! Pulse UI: Event dispatch, interactive tab switching, and state machine.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use pulse_core::{
    CpuTelemetry, DiskTelemetry, FileSystemManager, MemoryTelemetry, NetworkTelemetry,
    ProcessManager,
};
use pulse_render::{ActiveTab, PulseCanvas, PulseRenderer, Rect};

pub struct PulseUiState {
    pub proc_mgr: ProcessManager,
    pub cpu: CpuTelemetry,
    pub mem: MemoryTelemetry,
    pub net: NetworkTelemetry,
    pub disk: DiskTelemetry,
    pub fs_mgr: FileSystemManager,
    pub renderer: PulseRenderer,
    pub tick_counter: u64,
}

impl PulseUiState {
    pub fn new() -> Self {
        Self {
            proc_mgr: ProcessManager::new(),
            cpu: CpuTelemetry::new(4),
            mem: MemoryTelemetry::new(),
            net: NetworkTelemetry::new(),
            disk: DiskTelemetry::new(),
            fs_mgr: FileSystemManager::new(),
            renderer: PulseRenderer::new(),
            tick_counter: 0,
        }
    }

    pub fn switch_tab(&mut self, tab: ActiveTab) {
        self.renderer.active_tab = tab;
        self.renderer.show_details_modal = false;
    }

    pub fn select_pid(&mut self, pid: u32) {
        self.renderer.selected_pid = Some(pid);
    }

    pub fn open_details(&mut self) {
        if self.renderer.selected_pid.is_some() {
            self.renderer.show_details_modal = true;
        }
    }

    pub fn close_modal(&mut self) {
        self.renderer.show_details_modal = false;
    }

    pub fn end_selected_process(&mut self) -> Result<String, String> {
        if let Some(pid) = self.renderer.selected_pid {
            let res = self.proc_mgr.kill_process(pid);
            if res.is_ok() {
                self.renderer.show_details_modal = false;
                self.renderer.selected_pid = self.proc_mgr.processes().first().map(|p| p.pid);
            }
            res
        } else {
            Err("No process currently selected".to_string())
        }
    }

    pub fn step_telemetry(&mut self) {
        self.tick_counter += 1;
        let t = self.tick_counter as f32;

        // Dynamic oscillation simulating real system activity
        let c1 = (7.1 + (t * 0.3).sin() * 5.0).clamp(1.0, 95.0);
        let c2 = (12.1 + (t * 0.2).cos() * 8.0).clamp(1.0, 95.0);
        let c3 = (12.0 + (t * 0.25).sin() * 7.0).clamp(1.0, 95.0);
        let c4 = (11.1 + (t * 0.15).cos() * 6.0).clamp(1.0, 95.0);
        self.cpu.push_sample(&[c1, c2, c3, c4]);

        let mem_used = 8_600_000_000 + ((t * 0.1).sin() * 200_000_000.0) as u64;
        let swap_used = 2_100_000_000 + ((t * 0.05).cos() * 50_000_000.0) as u64;
        self.mem.push_sample(mem_used, swap_used);

        let net_recv = if self.tick_counter % 5 == 0 { 4096 } else { 0 };
        let net_send = if self.tick_counter % 8 == 0 { 2048 } else { 0 };
        self.net.push_sample(net_recv, net_send);

        let d_read = if self.tick_counter % 12 == 0 { 8_192_000 } else { 0 };
        let d_write = 536_000;
        self.disk.push_sample(d_read, d_write);
    }

    pub fn handle_click(&mut self, px: i32, py: i32, bounds: Rect) -> bool {
        // Modal dialog interception
        if self.renderer.show_details_modal {
            let card_w = 420.min(bounds.w - 32);
            let card_h = 360.min(bounds.h - 32);
            let cx = bounds.x + (bounds.w - card_w) / 2;
            let cy = bounds.y + (bounds.h - card_h) / 2;
            let card = Rect::new(cx, cy, card_w, card_h);

            // Close button [x]
            let close_btn = Rect::new(card.right() - 28, card.y + 10, 18, 18);
            if close_btn.contains(px, py) {
                self.close_modal();
                return true;
            }

            // End Process button inside modal
            let btn_end = Rect::new(card.x + 16, card.bottom() - 34, 110, 24);
            if btn_end.contains(px, py) {
                let _ = self.end_selected_process();
                return true;
            }

            // Click outside modal dismisses it
            if !card.contains(px, py) {
                self.close_modal();
                return true;
            }
            return true;
        }

        // Toolbar Tabs
        let th_h = 36;
        if py >= bounds.y && py < bounds.y + th_h {
            let tab_w = 110;
            let total_tabs_w = tab_w * 3;
            let tabs_start_x = bounds.x + (bounds.w - total_tabs_w) / 2;

            if px >= tabs_start_x && px < tabs_start_x + tab_w {
                self.switch_tab(ActiveTab::Processes);
                return true;
            } else if px >= tabs_start_x + tab_w && px < tabs_start_x + tab_w * 2 {
                self.switch_tab(ActiveTab::Resources);
                return true;
            } else if px >= tabs_start_x + tab_w * 2 && px < tabs_start_x + total_tabs_w {
                self.switch_tab(ActiveTab::FileSystems);
                return true;
            }
        }

        // Processes Tab Clicks
        if self.renderer.active_tab == ActiveTab::Processes {
            let client_top = bounds.y + th_h + 24; // Below header
            let client_bottom = bounds.bottom() - 32; // Above toolbar
            let row_h = 22;

            if py >= client_top && py < client_bottom {
                let row_idx = ((py - client_top) / row_h) as usize;
                let list = self.proc_mgr.filter_by_query(&self.renderer.search_query);
                if let Some(p) = list.get(row_idx) {
                    if self.renderer.selected_pid == Some(p.pid) {
                        // Double-click / re-click opens modal
                        self.open_details();
                    } else {
                        self.select_pid(p.pid);
                    }
                    return true;
                }
            }

            // Bottom [ End Process ] button
            let btn_end = Rect::new(bounds.x + 8, bounds.bottom() - 28, 100, 24);
            if btn_end.contains(px, py) {
                let _ = self.end_selected_process();
                return true;
            }
        }

        false
    }

    pub fn handle_key(&mut self, key: char) -> bool {
        match key {
            '1' => {
                self.switch_tab(ActiveTab::Processes);
                true
            }
            '2' => {
                self.switch_tab(ActiveTab::Resources);
                true
            }
            '3' => {
                self.switch_tab(ActiveTab::FileSystems);
                true
            }
            '\n' | ' ' => {
                if self.renderer.show_details_modal {
                    self.close_modal();
                } else if self.renderer.selected_pid.is_some() {
                    self.open_details();
                }
                true
            }
            '\x1b' => {
                // ESC
                if self.renderer.show_details_modal {
                    self.close_modal();
                    true
                } else {
                    false
                }
            }
            'k' | 'K' => {
                let _ = self.end_selected_process();
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, canvas: &mut PulseCanvas, bounds: Rect) {
        self.renderer.render(
            canvas,
            bounds,
            &self.proc_mgr,
            &self.cpu,
            &self.mem,
            &self.net,
            &self.disk,
            &self.fs_mgr,
        );
    }
}

impl Default for PulseUiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_state_workflow() {
        let mut state = PulseUiState::new();
        state.proc_mgr.populate_default_telemetry();

        // 1. Initial tab is Processes
        assert_eq!(state.renderer.active_tab, ActiveTab::Processes);

        // 2. Switch tabs via method and keys
        state.switch_tab(ActiveTab::Resources);
        assert_eq!(state.renderer.active_tab, ActiveTab::Resources);

        state.handle_key('3');
        assert_eq!(state.renderer.active_tab, ActiveTab::FileSystems);

        state.handle_key('1');
        assert_eq!(state.renderer.active_tab, ActiveTab::Processes);

        // 3. Selection & Modal
        state.select_pid(5259);
        assert_eq!(state.renderer.selected_pid, Some(5259));

        state.open_details();
        assert!(state.renderer.show_details_modal);

        state.handle_key('\x1b');
        assert!(!state.renderer.show_details_modal);

        // 4. Telemetry stepping
        state.step_telemetry();
        assert_eq!(state.tick_counter, 1);

        // 5. Termination
        let initial_count = state.proc_mgr.processes().len();
        state.select_pid(4456);
        let res = state.end_selected_process();
        assert!(res.is_ok());
        assert_eq!(state.proc_mgr.processes().len(), initial_count - 1);
    }
}
