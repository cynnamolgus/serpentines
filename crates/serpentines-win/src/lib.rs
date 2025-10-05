//! Windows platform implementations (stubs) for Serpentines.

use serpentines_platform::{ControlPanel, GpuRenderer, InputSource, MonitorRect, OverlayManager, Result, SystemTray};
use tracing::{info, warn};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Graphics::Gdi::HBRUSH;

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent, MouseButton};
use serpentines_ui::{spawn_ui_thread, UiCommand};
use crossbeam_channel::Sender as CbSender;
use std::sync::{Arc, Mutex};

// Public app entry ----------------
/// Start the Windows message loop, create tray icon, overlay, and a placeholder control panel.
pub fn run_app() -> Result<()> {
    unsafe {
        let hinstance = HINSTANCE(GetModuleHandleW(None)?.0);
        let overlay_class = register_window_class(hinstance, windows::core::w!("SerpentinesOverlayClass"), Some(overlay_wnd_proc));

        // Spawn egui UI on a separate thread
        let ui_handles = spawn_ui_thread();
        let ui_command_sender_cell: Arc<Mutex<CbSender<UiCommand>>> = Arc::new(Mutex::new(ui_handles.command_sender.clone()));
        let overlay_hwnd = create_overlay_window(hinstance, overlay_class);

        // Create tray icon + menu via tray-icon crate
        let (tray, settings_id, exit_id) = create_tray_icon()?;
        info!("tray icon created; wiring event handlers");

        // Route tray icon clicks and menu selections
        handle_tray_icon_events(&ui_command_sender_cell);
        handle_tray_menu_events(&ui_command_sender_cell, &settings_id, &exit_id);

        let mut message = MSG::default();
        'main: loop {
            while PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).into() {
                if message.message == WM_QUIT { break 'main; }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            // Efficient wait for the next message. Tray handlers run via callbacks.
            let _ = WaitMessage();
        }

        // Cleanup
        drop(tray);
        let _ = DestroyWindow(overlay_hwnd);
    }
    Ok(())
}

fn handle_tray_icon_events(ui_command_sender_cell: &Arc<Mutex<CbSender<UiCommand>>>) {
    let ui_sender_cell = Arc::clone(ui_command_sender_cell);
    TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        match event {
            TrayIconEvent::Click { button, .. } if button == MouseButton::Left => {
                show_ui(&ui_sender_cell);
            }
            _ => {}
        }
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

// ---------------- Overlay manager (still stubs, but uses real overlay creation above) ----------------

pub struct WinOverlayManager;
impl WinOverlayManager {
    pub fn new() -> Self { Self }
}
impl OverlayManager for WinOverlayManager {
    fn init(&mut self) -> Result<()> { info!("WinOverlayManager init"); Ok(()) }
    fn monitors(&self) -> Result<Vec<MonitorRect>> {
        // TODO: enumerate real monitors. For now, primary monitor size.
        unsafe {
            let hwnd = GetDesktopWindow();
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            Ok(vec![MonitorRect { x: 0, y: 0, width, height, dpi: 96 }])
        }
    }
    fn create_overlays(&mut self) -> Result<()> { info!("create_overlays called"); Ok(()) }
    fn destroy_overlays(&mut self) -> Result<()> { info!("destroy_overlays called"); Ok(()) }
}

// ---------------- Input and GPU stubs (unchanged behavior) ----------------

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

// ---------------- Win32 helpers, tray helpers, and window procedures ----------------

unsafe fn register_window_class(hinstance: HINSTANCE, class_name: PCWSTR, wndproc: WNDPROC) -> PCWSTR {
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: wndproc,
        hInstance: hinstance,
        hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(std::ptr::null_mut()),
        lpszClassName: class_name,
        ..Default::default()
    };
    let _atom = RegisterClassW(&wc);
    class_name
}

// Removed hidden host window; egui handles UI

unsafe fn create_overlay_window(hinstance: HINSTANCE, class_name: PCWSTR) -> HWND {
    // Transparent, topmost, click-through overlay covering the desktop
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0),
        class_name,
        windows::core::w!("Serpentines Overlay"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        hinstance,
        None,
    ).expect("CreateWindowExW (overlay)");

    // Size it to the desktop
    let desktop = GetDesktopWindow();
    let mut rect = RECT::default();
    let _ = GetWindowRect(desktop, &mut rect);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        SWP_SHOWWINDOW,
    );

    // Layered window alpha 255 but fully transparent via painting; here we just ensure it's visible.
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);

    hwnd
}

// --------------- tray-icon integration ---------------

fn create_tray_icon() -> Result<(tray_icon::TrayIcon, MenuId, MenuId)> {
    // Placeholder 16x16 white square icon
    let (w, h) = (16, 16);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for px in rgba.chunks_exact_mut(4) { px.copy_from_slice(&[255, 255, 255, 255]); }
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
fn box_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(e)
}
 
// Removed host window procs; only overlay window remains

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize), // pass mouse through
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
