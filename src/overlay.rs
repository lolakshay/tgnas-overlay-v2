use anyhow::{anyhow, Result};
use std::mem::size_of;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

const CLASS_NAME: PCWSTR = w!("System.Dwm.FrameHost");
pub struct Overlay {
    hwnd: HWND,
    visible: bool,
}
impl Overlay {
    pub unsafe fn create(initially_visible: bool, topmost: bool) -> Result<Self> {
        // Do not cover the taskbar: Windows still treats a fully transparent topmost
        // window as an occupant of its rectangle and can otherwise disturb taskbar UX.
        let mut work_area = RECT::default();
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&raw mut work_area).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(window_proc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return Err(anyhow!(
                "RegisterClassExW failed: {}",
                windows::core::Error::from_thread()
            ));
        }
        let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
        let hwnd = CreateWindowExW(
            ex,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            work_area.left,
            work_area.top,
            work_area.right - work_area.left,
            work_area.bottom - work_area.top,
            None,
            None,
            Some(instance),
            None,
        )?;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA)
            .ok()
            .ok_or_else(|| anyhow!("SetLayeredWindowAttributes failed"))?;
        let insert_after = if topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        SetWindowPos(
            hwnd,
            Some(insert_after),
            work_area.left,
            work_area.top,
            work_area.right - work_area.left,
            work_area.bottom - work_area.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .ok()
        .ok_or_else(|| anyhow!("SetWindowPos failed"))?;
        let affinity = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        if affinity.is_err() {
            tracing::warn!("Capture exclusion unavailable: {}", affinity.unwrap_err());
        } else {
            tracing::info!("Capture exclusion enabled");
        }
        let mut overlay = Self {
            hwnd,
            visible: true,
        };
        if !initially_visible {
            overlay.set_visible(false)?;
        }
        Ok(overlay)
    }
    pub fn toggle(&mut self) -> Result<()> {
        self.set_visible(!self.visible)
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn set_visible(&mut self, visible: bool) -> Result<()> {
        unsafe {
            if visible {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = SetWindowDisplayAffinity(self.hwnd, WDA_EXCLUDEFROMCAPTURE);
            } else {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
        self.visible = visible;
        tracing::info!(visible, "Overlay state changed");
        Ok(())
    }
}
impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let _ = hdc;
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
