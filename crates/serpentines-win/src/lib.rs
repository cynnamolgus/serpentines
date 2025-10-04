//! Windows platform implementations (stubs) for Serpentines.

use serpentines_platform::{ControlPanel, GpuRenderer, InputSource, MonitorRect, OverlayManager, Result, SystemTray};
use tracing::{info, warn};

pub struct WinOverlayManager;
impl WinOverlayManager { pub fn new() -> Self { Self } }
impl OverlayManager for WinOverlayManager {
    fn init(&mut self) -> Result<()> { info!("WinOverlayManager init (stub)"); Ok(()) }
    fn monitors(&self) -> Result<Vec<MonitorRect>> {
        // Stub: single 1920x1080 monitor at 100% scaling
        Ok(vec![MonitorRect { x: 0, y: 0, width: 1920, height: 1080, dpi: 96 }])
    }
    fn create_overlays(&mut self) -> Result<()> { info!("create_overlays (stub)"); Ok(()) }
    fn destroy_overlays(&mut self) -> Result<()> { info!("destroy_overlays (stub)"); Ok(()) }
}

pub struct WinInputSource;
impl WinInputSource { pub fn new() -> Self { Self } }
impl InputSource for WinInputSource {
    fn start(&mut self) -> Result<()> { info!("Input hook start (stub)"); Ok(()) }
    fn stop(&mut self) -> Result<()> { info!("Input hook stop (stub)"); Ok(()) }
}

pub struct WinGpuRenderer;
impl WinGpuRenderer { pub fn new() -> Self { Self } }
impl GpuRenderer for WinGpuRenderer {
    fn init(&mut self) -> Result<()> { info!("wgpu init (stub)"); Ok(()) }
    fn render_frame(&mut self) -> Result<()> { warn!("render_frame (stub): no drawing yet"); Ok(()) }
}

pub struct WinSystemTray;
impl WinSystemTray { pub fn new() -> Self { Self } }
impl SystemTray for WinSystemTray {
    fn init(&mut self) -> Result<()> { info!("System tray init (stub)"); Ok(()) }
}

pub struct WinControlPanel;
impl WinControlPanel { pub fn new() -> Self { Self } }
impl ControlPanel for WinControlPanel {
    fn open(&mut self) -> Result<()> { info!("Control panel open (stub)"); Ok(()) }
    fn close(&mut self) -> Result<()> { info!("Control panel close (stub)"); Ok(()) }
}
