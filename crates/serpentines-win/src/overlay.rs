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
    hwnd_value: isize,
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
    fn hwnd(&self) -> HWND {
        HWND(self.hwnd_value as *mut c_void)
    }
}

pub struct WinOverlayManager {
    hinstance_value: isize,
    class_name_ptr: isize,
    overlays: HashMap<isize, DisplayOverlay>,
    monitor_rects: Vec<MonitorRect>,
}

impl WinOverlayManager {
    pub fn new(hinstance: HINSTANCE) -> Self {
        let class_name = unsafe { register_overlay_window_class(hinstance) };
        Self {
            hinstance_value: hinstance.0 as isize,
            class_name_ptr: class_name.0 as isize,
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
                        update_overlay_window(current.hwnd(), rect)?;
                    }
                    current.rect = *rect;
                }
                None => {
                    let hwnd = unsafe {
                        create_overlay_hwnd(
                            self as *mut WinOverlayManager,
                            self.hinstance_value,
                            self.class_name_ptr,
                            rect,
                        )?
                    };
                    self.overlays.insert(
                        *id,
                        DisplayOverlay {
                            hwnd_value: hwnd.0 as isize,
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
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprc: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let data_ptr = lparam.0 as *mut Vec<(isize, MonitorRect)>;
            if data_ptr.is_null() {
                return BOOL(0);
            }
            let data = &mut *data_ptr;
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(hmonitor, &mut info).as_bool() {
                return BOOL(1);
            }
            let rect = info.rcMonitor;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let mut dpi_x = 96u32;
            let mut dpi_y = 96u32;
            if let Err(err) = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y)
            {
                warn!(
                    "GetDpiForMonitor failed for monitor {:?}: {err}",
                    hmonitor.0
                );
                dpi_x = 96;
            }
            data.push((
                hmonitor.0 as isize,
                MonitorRect {
                    x: rect.left,
                    y: rect.top,
                    width,
                    height,
                    dpi: dpi_x,
                },
            ));
            BOOL(1)
        }

        let mut monitors: Vec<(isize, MonitorRect)> = Vec::new();
        let lparam = LPARAM(&mut monitors as *mut _ as isize);
        unsafe {
            let result = EnumDisplayMonitors(None, None, Some(enum_proc), lparam);
            if result == BOOL(0) {
                return Err(Error::from_win32().into());
            }
        }
        Ok(monitors)
    }

    unsafe fn destroy_overlay(&self, id: isize, overlay: DisplayOverlay) {
        SetWindowLongPtrW(overlay.hwnd(), GWLP_USERDATA, 0);
        if let Err(err) = DestroyWindow(overlay.hwnd()) {
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
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_DEVICECHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            let manager_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WinOverlayManager;
            if !manager_ptr.is_null() {
                let manager = &mut *manager_ptr;
                let reason = match msg {
                    WM_DEVICECHANGE => match wparam.0 as u32 {
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
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn create_overlay_hwnd(
    manager_ptr: *mut WinOverlayManager,
    hinstance_value: isize,
    class_name_ptr: isize,
    rect: &MonitorRect,
) -> Result<HWND> {
    let hinstance = HINSTANCE(hinstance_value as *mut core::ffi::c_void);
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(
            WS_EX_LAYERED.0
                | WS_EX_TRANSPARENT.0
                | WS_EX_TOPMOST.0
                | WS_EX_TOOLWINDOW.0
                | WS_EX_NOACTIVATE.0,
        ),
        PCWSTR(class_name_ptr as *const u16),
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
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, manager_ptr as isize);
    update_overlay_window(hwnd, rect)?;
    SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
    Ok(hwnd)
}

unsafe fn update_overlay_window(hwnd: HWND, rect: &MonitorRect) -> Result<()> {
    let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        flags,
    )?;
    Ok(())
}
