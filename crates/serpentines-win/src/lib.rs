//! Windows platform implementations (stubs) for Serpentines.
use serpentines_platform::{GpuRenderer, InputSource, OverlayManager, Result, SystemTray};
use tracing::{info, warn};

mod overlay;
use crate::overlay::WinOverlayManager;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crossbeam_channel::Sender as CbSender;
use serpentines_ui::{spawn_ui_thread, UiCommand};
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIconBuilder, TrayIconEvent};

// Public app entry ----------------
/// Start the Windows message loop, create tray icon, overlay, and a placeholder control panel.
pub fn run_app() -> Result<()> {
    unsafe {
        let hinstance = HINSTANCE(GetModuleHandleW(None)?.0);
        // Spawn egui UI on a separate thread
        let ui_handles = spawn_ui_thread();
        let ui_command_sender_cell: Arc<Mutex<CbSender<UiCommand>>> =
            Arc::new(Mutex::new(ui_handles.command_sender.clone()));
        let mut overlay_manager = WinOverlayManager::new(hinstance);
        overlay_manager.create_overlays()?;
        overlay_manager.log_current_layout("initial overlay creation");

        // Create tray icon + menu via tray-icon crate
        let (tray, settings_id, exit_id) = create_tray_icon()?;
        info!("tray icon created; wiring event handlers");

        // Route tray icon clicks and menu selections
        handle_tray_icon_events(&ui_command_sender_cell);
        handle_tray_menu_events(&ui_command_sender_cell, &settings_id, &exit_id);

        let mut message = MSG::default();
        'main: loop {
            while PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).into() {
                if message.message == WM_QUIT {
                    break 'main;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            // Efficient wait for the next message. Tray handlers run via callbacks.
            let _ = WaitMessage();
        }

        // Cleanup
        drop(tray);
        overlay_manager.destroy_overlays()?;
    }
    Ok(())
}

fn handle_tray_icon_events(ui_command_sender_cell: &Arc<Mutex<CbSender<UiCommand>>>) {
    let ui_sender_cell = Arc::clone(ui_command_sender_cell);
    TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| match event {
        TrayIconEvent::Click { button, .. } if button == MouseButton::Left => {
            show_ui(&ui_sender_cell);
        }
        _ => {}
    }));
}

fn handle_tray_menu_events(
    ui_command_sender_cell: &Arc<Mutex<CbSender<UiCommand>>>,
    settings_menu_id: &MenuId,
    exit_menu_id: &MenuId,
) {
    let ui_sender_cell2 = Arc::clone(ui_command_sender_cell);
    let settings_id = settings_menu_id.clone();
    let exit_id = exit_menu_id.clone();
    MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id() == &settings_id {
            show_ui(&ui_sender_cell2);
        } else if event.id() == &exit_id {
            unsafe { PostQuitMessage(0) };
        }
    }));
}

fn show_ui(ui_sender_cell: &Arc<Mutex<CbSender<UiCommand>>>) {
    if let Ok(sender) = ui_sender_cell.lock() {
        match sender.send(UiCommand::Show) {
            Ok(()) => info!("win: sent UiCommand::Show ok"),
            Err(_) => warn!("UI command channel closed on Show; cannot show window"),
        }
    } else {
        warn!("Failed to lock UI command sender");
    }
}

// ---------------- Input and GPU stubs (unchanged behavior) ----------------

pub struct WinInputSource;
impl WinInputSource {
    pub fn new() -> Self {
        Self
    }
}
impl InputSource for WinInputSource {
    fn start(&mut self) -> Result<()> {
        info!("Input hook start (stub)");
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        info!("Input hook stop (stub)");
        Ok(())
    }
}

pub struct WinGpuRenderer;
impl WinGpuRenderer {
    pub fn new() -> Self {
        Self
    }
}
impl GpuRenderer for WinGpuRenderer {
    fn init(&mut self) -> Result<()> {
        info!("wgpu init (stub)");
        Ok(())
    }
    fn render_frame(&mut self) -> Result<()> {
        warn!("render_frame (stub): no drawing yet");
        Ok(())
    }
}

// --------------- tray-icon integration ---------------

fn create_tray_icon() -> Result<(tray_icon::TrayIcon, MenuId, MenuId)> {
    // Placeholder 16x16 white square icon
    let (w, h) = (16, 16);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 255, 255, 255]);
    }
    let icon = Icon::from_rgba(rgba, w, h).map_err(box_err)?;

    let menu = Menu::new();
    let settings_item = MenuItem::new("Open Settings", true, None);
    let exit_item = MenuItem::new("Exit", true, None);
    let settings_id = settings_item.id().clone();
    let exit_id = exit_item.id().clone();
    menu.append(&settings_item).map_err(box_err)?;
    menu.append(&exit_item).map_err(box_err)?;

    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("Serpentines")
        .with_menu(Box::new(menu))
        .build()
        .map_err(box_err)?;

    Ok((tray, settings_id, exit_id))
}

// Removed polling helpers; using set_event_handler callbacks instead

#[inline]
fn box_err<E: std::error::Error + Send + Sync + 'static>(
    e: E,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(e)
}
