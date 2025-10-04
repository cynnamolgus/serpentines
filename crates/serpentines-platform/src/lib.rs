//! Platform abstraction traits so `serpentines-core` stays OS-agnostic.

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub dpi: u32,
}

/// Manages per-monitor transparent overlays.
pub trait OverlayManager: Send + Sync {
    fn init(&mut self) -> Result<()>;
    fn monitors(&self) -> Result<Vec<MonitorRect>>;
    fn create_overlays(&mut self) -> Result<()>;
    fn destroy_overlays(&mut self) -> Result<()>;
}

/// Source of global mouse events.
pub trait InputSource: Send + Sync {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
}

/// GPU renderer abstraction (to be backed by wgpu on each platform).
pub trait GpuRenderer: Send + Sync {
    fn init(&mut self) -> Result<()>;
    fn render_frame(&mut self) -> Result<()>;
    fn resize(&mut self, _width: u32, _height: u32) -> Result<()> { Ok(()) }
}

/// System tray with context menu and activation.
pub trait SystemTray: Send + Sync {
    fn init(&mut self) -> Result<()>;
    fn show_message(&self, _title: &str, _body: &str) -> Result<()> { Ok(()) }
}

/// Control panel window (native UI) — kept separate from overlays.
pub trait ControlPanel: Send + Sync {
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
}
