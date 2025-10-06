use serpentines_platform::{MonitorRect, OverlayManager, Result};
use tracing::{info, warn};

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::mem::size_of;

use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::{BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HBRUSH, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const OVERLAY_WINDOW_CLASS_NAME: PCWSTR = windows::core::w!("SerpentinesOverlayClass");

struct DisplayOverlay {
    window_handle_value: isize,
    rect: MonitorRect,
}

unsafe fn register_overlay_window_class(hinstance: HINSTANCE) -> PCWSTR {
    let window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(handle_overlay_window_message),
        hInstance: hinstance,
        hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(std::ptr::null_mut()),
        lpszClassName: OVERLAY_WINDOW_CLASS_NAME,
        ..Default::default()
    };
    let _atom = RegisterClassW(&window_class);
    OVERLAY_WINDOW_CLASS_NAME
}

impl DisplayOverlay {
    fn window_handle(&self) -> HWND {
        HWND(self.window_handle_value as *mut c_void)
    }
}

pub struct WinOverlayManager {
    instance_handle_value: isize,
    class_name_pointer: isize,
    overlays: HashMap<isize, DisplayOverlay>,
    monitor_rects: Vec<MonitorRect>,
}

impl WinOverlayManager {
    pub fn new(hinstance: HINSTANCE) -> Self {
        let class_name = unsafe { register_overlay_window_class(hinstance) };
        Self {
            instance_handle_value: hinstance.0 as isize,
            class_name_pointer: class_name.0 as isize,
            overlays: HashMap::new(),
            monitor_rects: Vec::new(),
        }
    }

    pub fn refresh_overlays(&mut self) -> Result<()> {
        let monitors = Self::enumerate_monitors()?;
        let desired: HashSet<isize> = monitors.iter().map(|(id, _)| *id).collect();
        let existing_keys: Vec<isize> = self.overlays.keys().copied().collect();
        for key in existing_keys {
            if !desired.contains(&key) {
                if let Some(overlay) = self.overlays.remove(&key) {
                    unsafe {
                        self.destroy_overlay(key, overlay);
                    }
                }
            }
        }
        for (id, rect) in monitors.iter() {
            match self.overlays.get_mut(id) {
                Some(current) => {
                    unsafe {
                        update_overlay_window(current.window_handle(), rect)?;
                    }
                    current.rect = *rect;
                }
                None => {
                    let window_handle = unsafe {
                        create_overlay_hwnd(
                            self as *mut WinOverlayManager,
                            self.instance_handle_value,
                            self.class_name_pointer,
                            rect,
                        )?
                    };
                    self.overlays.insert(
                        *id,
                        DisplayOverlay {
                            window_handle_value: window_handle.0 as isize,
                            rect: *rect,
                        },
                    );
                }
            }
        }

        self.monitor_rects = monitors.iter().map(|(_, rect)| *rect).collect();
        info!(
            "Overlay refresh => {} displays: {}",
            self.monitor_rects.len(),
            self.monitor_rects
                .iter()
                .map(|r| format!(
                    "[{}x{} @ ({}, {}) dpi {}]",
                    r.width, r.height, r.x, r.y, r.dpi
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(())
    }

    pub fn log_current_layout(&self, reason: &str) {
        if self.monitor_rects.is_empty() {
            info!("Overlay layout ({}) -> no active displays", reason);
            return;
        }
        let mut details = Vec::new();
        for (index, rect) in self.monitor_rects.iter().enumerate() {
            details.push(format!(
                "display {}: origin=({}, {}), size={}x{}, dpi={}",
                index, rect.x, rect.y, rect.width, rect.height, rect.dpi
            ));
        }
        info!("Overlay layout ({}) -> {}", reason, details.join("; "));
    }

    pub fn handle_environment_change(&mut self, reason: &str) {
        match self.refresh_overlays() {
            Ok(()) => self.log_current_layout(reason),
            Err(err) => warn!("failed to refresh overlays after {reason}: {err}"),
        }
    }

    pub fn teardown_overlays(&mut self) -> Result<()> {
        let overlays = std::mem::take(&mut self.overlays);
        for (id, overlay) in overlays {
            unsafe {
                self.destroy_overlay(id, overlay);
            }
        }
        self.monitor_rects.clear();
        Ok(())
    }

    fn enumerate_monitors() -> Result<Vec<(isize, MonitorRect)>> {
        unsafe extern "system" fn enum_proc(
            monitor_handle: HMONITOR,
            _hdc: HDC,
            _lprc: *mut RECT,
            long_parameter: LPARAM,
        ) -> BOOL {
            let monitor_data_pointer = long_parameter.0 as *mut Vec<(isize, MonitorRect)>;
            if monitor_data_pointer.is_null() {
                return BOOL(0);
            }
            let monitor_data = &mut *monitor_data_pointer;
            let mut monitor_info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor_handle, &mut monitor_info).as_bool() {
                return BOOL(1);
            }
            let rect = monitor_info.rcMonitor;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let mut horizontal_dpi = 96u32;
            let mut vertical_dpi = 96u32;
            if let Err(err) = GetDpiForMonitor(monitor_handle, MDT_EFFECTIVE_DPI, &mut horizontal_dpi, &mut vertical_dpi) {
                warn!(
                    "GetDpiForMonitor failed for monitor {:?}: {err}",
                    monitor_handle.0
                );
                horizontal_dpi = 96;
            }
            monitor_data.push((
                monitor_handle.0 as isize,
                MonitorRect {
                    x: rect.left,
                    y: rect.top,
                    width,
                    height,
                    dpi: horizontal_dpi,
                },
            ));
            BOOL(1)
        }

        let mut monitors: Vec<(isize, MonitorRect)> = Vec::new();
        let long_parameter = LPARAM(&mut monitors as *mut _ as isize);
        unsafe {
            let result = EnumDisplayMonitors(None, None, Some(enum_proc), long_parameter);
            if result == BOOL(0) {
                return Err(Error::from_win32().into());
            }
        }
        Ok(monitors)
    }

    unsafe fn destroy_overlay(&self, id: isize, overlay: DisplayOverlay) {
        SetWindowLongPtrW(overlay.window_handle(), GWLP_USERDATA, 0);
        if let Err(err) = DestroyWindow(overlay.window_handle()) {
            warn!("DestroyWindow failed for overlay {id:?}: {err}");
        }
    }
}

impl OverlayManager for WinOverlayManager {
    fn init(&mut self) -> Result<()> {
        self.refresh_overlays()
    }

    fn monitors(&self) -> Result<Vec<MonitorRect>> {
        Ok(self.monitor_rects.clone())
    }

    fn create_overlays(&mut self) -> Result<()> {
        self.refresh_overlays()
    }

    fn destroy_overlays(&mut self) -> Result<()> {
        self.teardown_overlays()
    }
}

pub unsafe extern "system" fn handle_overlay_window_message(
    window_handle: HWND,
    message: u32,
    word_parameter: WPARAM,
    long_parameter: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_DEVICECHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            let manager_ptr = GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut WinOverlayManager;
            if !manager_ptr.is_null() {
                let manager = &mut *manager_ptr;
                let reason = match message {
                    WM_DEVICECHANGE => match word_parameter.0 as u32 {
                        DBT_DEVICEARRIVAL => "device arrival",
                        DBT_DEVICEREMOVECOMPLETE => "device removal",
                        DBT_DEVNODES_CHANGED => "device nodes changed",
                        _ => "device change",
                    },
                    WM_DISPLAYCHANGE => "display change",
                    WM_DPICHANGED => "dpi change",
                    WM_SETTINGCHANGE => "setting change",
                    _ => "environment change",
                };
                manager.handle_environment_change(reason);
            }
            DefWindowProcW(window_handle, message, word_parameter, long_parameter)
        }
        WM_DESTROY => {
            SetWindowLongPtrW(window_handle, GWLP_USERDATA, 0);
            DefWindowProcW(window_handle, message, word_parameter, long_parameter)
        }
        _ => DefWindowProcW(window_handle, message, word_parameter, long_parameter),
    }
}

unsafe fn create_overlay_hwnd(
    manager_pointer: *mut WinOverlayManager,
    instance_handle_value: isize,
    class_name_pointer: isize,
    rect: &MonitorRect,
) -> Result<HWND> {
    let hinstance = HINSTANCE(instance_handle_value as *mut core::ffi::c_void);
    let window_handle = CreateWindowExW(
        WINDOW_EX_STYLE(
            WS_EX_LAYERED.0
                | WS_EX_TRANSPARENT.0
                | WS_EX_TOPMOST.0
                | WS_EX_TOOLWINDOW.0
                | WS_EX_NOACTIVATE.0,
        ),
        PCWSTR(class_name_pointer as *const u16),
        windows::core::w!("Serpentines Overlay"),
        WS_POPUP,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        None,
        None,
        hinstance,
        None,
    )?;
    SetWindowLongPtrW(window_handle, GWLP_USERDATA, manager_pointer as isize);
    update_overlay_window(window_handle, rect)?;
    SetLayeredWindowAttributes(window_handle, COLORREF(0), 255, LWA_ALPHA)?;
    Ok(window_handle)
}

unsafe fn update_overlay_window(window_handle: HWND, rect: &MonitorRect) -> Result<()> {
    let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
    SetWindowPos(
        window_handle,
        HWND_TOPMOST,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        flags,
    )?;
    Ok(())
}
