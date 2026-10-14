use anyhow::{anyhow, Result};
use std::mem::size_of;
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
            SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DT_LEFT,
            DT_SINGLELINE, DT_VCENTER, FW_BOLD, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

const CLASS: PCWSTR = w!("System.Dwm.StatusView");
const COLOR_KEY: COLORREF = COLORREF(0x00000000); // Pure black transparent color key (no background)

pub struct StatusBadge {
    hwnd: HWND,
    visible: bool,
}
impl StatusBadge {
    pub unsafe fn create(
        text: &str,
        show_configured: bool,
        initially_visible: bool,
    ) -> Result<Option<Self>> {
        if !show_configured {
            return Ok(None);
        }
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: CLASS,
            lpfnWndProc: Some(proc),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 && windows::core::Error::from_thread().code().0 != 1410 {
            return Err(anyhow!("RegisterClassExW failed"));
        }
        let value = Box::new(HSTRING::from(text));
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!(""),
            WS_POPUP,
            16,
            16,
            240,
            32,
            None,
            None,
            Some(instance),
            Some(Box::into_raw(value).cast()),
        )?;

        // LWA_COLORKEY renders COLOR_KEY (pure black) as 100% transparent: NO background box!
        SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY)
            .ok()
            .ok_or_else(|| anyhow!("SetLayeredWindowAttributes failed"))?;

        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE).ok();
        let show_flag = if initially_visible {
            SWP_SHOWWINDOW
        } else {
            SWP_HIDEWINDOW
        };
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            16,
            16,
            240,
            32,
            SWP_NOACTIVATE | show_flag,
        )
        .ok()
        .ok_or_else(|| anyhow!("SetWindowPos failed"))?;
        Ok(Some(Self {
            hwnd,
            visible: initially_visible,
        }))
    }

    pub fn set_visible(&mut self, visible: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
        self.visible = visible;
    }

    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}
impl Drop for StatusBadge {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            LRESULT(1)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            // Fill window with COLOR_KEY (pure black), which is 100% transparent.
            // Result: Zero background, no box, no border!
            let bg_brush = CreateSolidBrush(COLOR_KEY);
            let _ = FillRect(hdc, &client_rect, bg_brush);
            let _ = DeleteObject(bg_brush.into());

            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const HSTRING;
            if !ptr.is_null() {
                let text = &*ptr;
                let text_str = text.to_string();
                if !text_str.is_empty() {
                    let font = CreateFontW(
                        16,
                        0,
                        0,
                        0,
                        FW_BOLD.0 as i32,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET,
                        OUT_TT_PRECIS,
                        windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                        CLEARTYPE_QUALITY,
                        0,
                        w!("Segoe UI"),
                    );
                    let old_font = SelectObject(hdc, font.into());
                    let _ = SetBkMode(hdc, TRANSPARENT);

                    let mut characters: Vec<u16> = text_str.encode_utf16().collect();

                    // 1. Draw 8-way subtle dark outline/halo (RGB 18, 18, 18 -> 0x00121212)
                    //    Ensures the floating badge is clearly readable on both light and dark screens
                    let _ = SetTextColor(hdc, COLORREF(0x00121212));
                    let offsets = [
                        (-1, -1), (0, -1), (1, -1),
                        (-1,  0),          (1,  0),
                        (-1,  1), (0,  1), (1,  1),
                        (2, 2),
                    ];
                    for (dx, dy) in offsets {
                        let mut shadow_rect = RECT {
                            left: client_rect.left + dx,
                            top: client_rect.top + dy,
                            right: client_rect.right + dx,
                            bottom: client_rect.bottom + dy,
                        };
                        let mut shadow_chars = characters.clone();
                        let _ = DrawTextW(
                            hdc,
                            &mut shadow_chars,
                            &raw mut shadow_rect,
                            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                        );
                    }

                    // 2. Draw crisp foreground text in bright white (RGB 255, 255, 255)
                    let _ = SetTextColor(hdc, COLORREF(0x00FFFFFF));
                    let mut main_rect = client_rect;
                    let _ = DrawTextW(
                        hdc,
                        &mut characters,
                        &raw mut main_rect,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    );

                    let _ = SelectObject(hdc, old_font);
                    let _ = DeleteObject(font.into());
                }
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut HSTRING;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
