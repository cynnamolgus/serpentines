use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use serpentines_win::{
    WinControlPanel, WinGpuRenderer, WinInputSource, WinOverlayManager, WinSystemTray,
};
use serpentines_platform::{ControlPanel, GpuRenderer, InputSource, OverlayManager, SystemTray};

fn main() {
    // Init logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("info")
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Serpentines starting (stub scaffolding)");

    // Initialize platform subsystems (stubs for now)
    let mut overlay = WinOverlayManager::new();
    let mut renderer = WinGpuRenderer::new();
    let mut input = WinInputSource::new();
    let mut tray = WinSystemTray::new();
    let mut panel = WinControlPanel::new();

    // In a real app these would be intertwined with a message loop.
    overlay.init().ok();
    let _monitors = overlay.monitors().unwrap_or_default();
    overlay.create_overlays().ok();

    renderer.init().ok();
    input.start().ok();
    tray.init().ok();
    panel.open().ok();

    // Placeholder main loop (single iteration)
    renderer.render_frame().ok();

    // Cleanup stubs
    input.stop().ok();
    overlay.destroy_overlays().ok();

    info!("Serpentines exiting (stub)");
}
