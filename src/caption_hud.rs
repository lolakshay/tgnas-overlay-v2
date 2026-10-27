use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
            FillRect, FrameRect, GetDC, InvalidateRect, ReleaseDC, SelectObject, SetBkMode,
            SetTextColor, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DT_CALCRECT, DT_CENTER,
            DT_VCENTER, DT_WORDBREAK, FW_BOLD, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

const CLASS_NAME: PCWSTR = w!("System.Dwm.SurfaceHost");
const COLOR_KEY: COLORREF = COLORREF(0x00000000); // Pure black is the transparent key (no background)

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum HudTheme {
    #[default]
    GhostWhite,
    MatrixGreen,
    CyberAmber,
    StealthGray,
}

impl HudTheme {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "green" | "matrix" | "ide green" => HudTheme::MatrixGreen,
            "amber" | "cyber" | "ide amber" => HudTheme::CyberAmber,
            "gray" | "grey" | "stealth" | "low-contrast" | "low-contrast gray" => HudTheme::StealthGray,
            _ => HudTheme::GhostWhite,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            HudTheme::GhostWhite => "white",
            HudTheme::MatrixGreen => "green",
            HudTheme::CyberAmber => "amber",
            HudTheme::StealthGray => "gray",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            HudTheme::GhostWhite => "White",
            HudTheme::MatrixGreen => "Green",
            HudTheme::CyberAmber => "Amber",
            HudTheme::StealthGray => "Gray",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HudTheme::GhostWhite => "Ghost White",
            HudTheme::MatrixGreen => "IDE Green",
            HudTheme::CyberAmber => "IDE Amber",
            HudTheme::StealthGray => "Low-Contrast Gray",
        }
    }

    pub fn text_color(&self) -> COLORREF {
        match self {
            HudTheme::GhostWhite => COLORREF(0x00FFFFFF), // Bright crisp white
            HudTheme::MatrixGreen => COLORREF(0x0050FF50), // Terminal green
            HudTheme::CyberAmber => COLORREF(0x0020B0FF),  // Amber (RGB 255, 176, 32 -> BGR 0x0020B0FF)
            HudTheme::StealthGray => COLORREF(0x00A0A0A0), // Stealth low contrast
        }
    }

    pub fn shadow_color(&self) -> COLORREF {
        match self {
            HudTheme::MatrixGreen => COLORREF(0x00081C08),
            HudTheme::CyberAmber => COLORREF(0x00101828),
            HudTheme::StealthGray => COLORREF(0x00141414),
            HudTheme::GhostWhite => COLORREF(0x00121212),
        }
    }

    pub fn next(&self) -> Self {
        match self {
            HudTheme::GhostWhite => HudTheme::MatrixGreen,
            HudTheme::MatrixGreen => HudTheme::CyberAmber,
            HudTheme::CyberAmber => HudTheme::StealthGray,
            HudTheme::StealthGray => HudTheme::GhostWhite,
        }
    }
}

struct HudSharedState {
    text: String,
    drag_mode: bool,
    opacity: u8,
    font_size: i32,
    theme: HudTheme,
    last_active: Instant,
    auto_dim_enabled: bool,
    is_dimmed: bool,
}

pub struct CaptionHud {
    hwnd: HWND,
    visible: bool,
    state: Arc<Mutex<HudSharedState>>,
}

impl CaptionHud {
    pub unsafe fn create(
        initially_visible: bool,
        opacity: u8,
        custom_rect: Option<(i32, i32, i32, i32)>,
        font_size: i32,
        theme: HudTheme,
        auto_dim_enabled: bool,
    ) -> Result<Self> {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 && windows::core::Error::from_thread().code().0 != 1410 {
            return Err(anyhow!("RegisterClassExW for CaptionHUD failed"));
        }

        let mut work_area = RECT::default();
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&raw mut work_area).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;

        let screen_w = work_area.right - work_area.left;
        let screen_h = work_area.bottom - work_area.top;
        let default_w = 800.min(screen_w - 60);
        let default_h = 76;
        let default_x = work_area.left + (screen_w - default_w) / 2;
        // Anchor into the lower-middle third of the screen (~68% down the viewport)
        // Keeps the text right beneath the active question area to minimize eye travel!
        let default_y = work_area.top + (screen_h * 68 / 100) - (default_h / 2);

        let (hud_x, hud_y, hud_w, hud_h) = match custom_rect {
            Some((x, y, w, h)) if w >= 200 && h >= 40 => (x, y, w, h),
            _ => (default_x, default_y, default_w, default_h),
        };

        let state = Arc::new(Mutex::new(HudSharedState {
            text: "Ready (Press P + O to capture & solve)".to_string(),
            drag_mode: false,
            opacity,
            font_size: font_size.clamp(14, 38),
            theme,
            last_active: Instant::now(),
            auto_dim_enabled,
            is_dimmed: false,
        }));
        let raw_state = Arc::into_raw(state.clone());

        // WS_EX_TRANSPARENT ensures mouse clicks pass 100% straight through to underlying windows!
        // WS_EX_TOPMOST ensures the window stays in the topmost band
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            hud_x,
            hud_y,
            hud_w,
            hud_h,
            None,
            None,
            Some(instance),
            Some(raw_state as *const std::ffi::c_void),
        )?;

        // LWA_COLORKEY renders COLOR_KEY (pure black) as 100% transparent.
        // Combining with LWA_ALPHA controls floating text opacity smoothly!
        SetLayeredWindowAttributes(hwnd, COLOR_KEY, opacity, LWA_COLORKEY | LWA_ALPHA)
            .ok()
            .ok_or_else(|| anyhow!("SetLayeredWindowAttributes failed for CaptionHUD"))?;

        // Exclude completely from screenshots & screen recordings
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);

        // Start 75ms interval timer for auto-dim, hover wake-up, and continuous topmost Z-order enforcement
        // (Beats Proctorio's 150ms focus loop to guarantee the HUD is always physically on top)
        let _ = SetTimer(Some(hwnd), 1, 75, None);

        let show_flag = if initially_visible {
            SWP_SHOWWINDOW
        } else {
            SWP_HIDEWINDOW
        };
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            hud_x,
            hud_y,
            hud_w,
            hud_h,
            SWP_NOACTIVATE | show_flag,
        )
        .ok()
        .ok_or_else(|| anyhow!("SetWindowPos failed for CaptionHUD"))?;

        Ok(Self {
            hwnd,
            visible: initially_visible,
            state,
        })
    }

    pub fn wake(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.last_active = Instant::now();
            if state.is_dimmed {
                state.is_dimmed = false;
                let opacity = state.opacity;
                unsafe {
                    let _ = SetLayeredWindowAttributes(self.hwnd, COLOR_KEY, opacity, LWA_COLORKEY | LWA_ALPHA);
                    let _ = InvalidateRect(Some(self.hwnd), None, true);
                }
            }
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.wake();
        let font_size = if let Ok(mut state) = self.state.lock() {
            state.text = text.to_string();
            state.font_size
        } else {
            22
        };

        // Dynamically compute required window height to prevent any text clipping on long answers/code!
        unsafe {
            let hdc = GetDC(Some(self.hwnd));
            if !hdc.is_invalid() {
                let font = CreateFontW(
                    font_size, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
                    DEFAULT_CHARSET, OUT_TT_PRECIS,
                    windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                    CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
                );
                let old_font = SelectObject(hdc, font.into());

                let mut wr = RECT::default();
                let _ = GetWindowRect(self.hwnd, &mut wr);
                let cur_w = (wr.right - wr.left).max(300);

                let mut calc_rect = RECT {
                    left: 0,
                    top: 0,
                    right: cur_w - 36,
                    bottom: 0,
                };
                let mut wide_text: Vec<u16> = text.encode_utf16().collect();
                let _ = DrawTextW(hdc, &mut wide_text, &mut calc_rect, DT_CALCRECT | DT_WORDBREAK);
                let needed_h = (calc_rect.bottom - calc_rect.top + 32).clamp(72, 420);

                let _ = SelectObject(hdc, old_font);
                let _ = DeleteObject(font.into());
                let _ = ReleaseDC(Some(self.hwnd), hdc);

                let new_y = (wr.bottom - needed_h).max(10);
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    wr.left,
                    new_y,
                    cur_w,
                    needed_h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = SetWindowDisplayAffinity(self.hwnd, WDA_EXCLUDEFROMCAPTURE);
            }
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.wake();
            unsafe {
                let _ = SetWindowDisplayAffinity(self.hwnd, WDA_EXCLUDEFROMCAPTURE);
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let _ = InvalidateRect(Some(self.hwnd), None, true);
            }
        } else {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
        self.visible = visible;
    }

    pub fn toggle_visible(&mut self) -> bool {
        let new_state = !self.visible;
        self.set_visible(new_state);
        new_state
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_opacity(&mut self, opacity: u8) {
        self.wake();
        if let Ok(mut state) = self.state.lock() {
            state.opacity = opacity;
        }
        unsafe {
            let _ = SetLayeredWindowAttributes(self.hwnd, COLOR_KEY, opacity, LWA_COLORKEY | LWA_ALPHA);
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    pub fn set_font_size(&mut self, font_size: i32) {
        self.wake();
        if let Ok(mut state) = self.state.lock() {
            state.font_size = font_size.clamp(14, 38);
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    #[allow(dead_code)]
    pub fn get_font_size(&self) -> i32 {
        if let Ok(state) = self.state.lock() {
            state.font_size
        } else {
            22
        }
    }

    pub fn cycle_theme(&mut self) -> HudTheme {
        self.wake();
        let new_theme = if let Ok(mut state) = self.state.lock() {
            state.theme = state.theme.next();
            state.theme
        } else {
            HudTheme::GhostWhite
        };
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
        new_theme
    }

    #[allow(dead_code)]
    pub fn get_theme(&self) -> HudTheme {
        if let Ok(state) = self.state.lock() {
            state.theme
        } else {
            HudTheme::GhostWhite
        }
    }

    pub fn set_drag_mode(&mut self, active: bool) -> (i32, i32, i32, i32) {
        self.wake();
        if let Ok(mut state) = self.state.lock() {
            state.drag_mode = active;
        }
        unsafe {
            let ex_style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            if active {
                // Clear WS_EX_TRANSPARENT so mouse clicks/drags are received
                let _ = SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_TRANSPARENT.0 as isize));
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            } else {
                // Restore WS_EX_TRANSPARENT for 100% click-through
                let _ = SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex_style | (WS_EX_TRANSPARENT.0 as isize));
            }
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED | SWP_NOACTIVATE,
            );
            let _ = InvalidateRect(Some(self.hwnd), None, true);

            let mut rect = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut rect);
            (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
        }
    }

    pub fn is_drag_mode(&self) -> bool {
        if let Ok(state) = self.state.lock() {
            state.drag_mode
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn get_rect(&self) -> (i32, i32, i32, i32) {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut rect);
            (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
        }
    }

    #[allow(dead_code)]
    pub fn get_hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for CaptionHud {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), 1);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            LRESULT(1)
        }
        WM_TIMER => {
            // Continuously pin window to top of topmost band to beat Proctorio's 150ms focus loop
            if IsWindowVisible(hwnd).as_bool() {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            }

            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<HudSharedState>;
            if !ptr.is_null() {
                let shared = &*ptr;

                // 1. Instantly wake up when mouse hovers over caption text area even in click-through mode!
                let mut pt = windows::Win32::Foundation::POINT::default();
                if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt).is_ok() {
                    let mut wr = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut wr);
                    if pt.x >= wr.left && pt.x <= wr.right && pt.y >= wr.top && pt.y <= wr.bottom {
                        if let Ok(mut state) = shared.lock() {
                            state.last_active = Instant::now();
                            if state.is_dimmed {
                                state.is_dimmed = false;
                                let normal_op = state.opacity;
                                let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, normal_op, LWA_COLORKEY | LWA_ALPHA);
                                let _ = InvalidateRect(Some(hwnd), None, true);
                            }
                        }
                    }
                }

                // 2. If inactive for >8 seconds and not in drag mode, dim to ~10% faint ghost opacity
                if let Ok(mut state) = shared.lock() {
                    if state.auto_dim_enabled && !state.drag_mode && !state.is_dimmed {
                        if state.last_active.elapsed() >= Duration::from_secs(8) {
                            state.is_dimmed = true;
                            let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, 25, LWA_COLORKEY | LWA_ALPHA);
                            let _ = InvalidateRect(Some(hwnd), None, true);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED | WM_SIZE => {
            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<HudSharedState>;
            let in_drag_mode = if !ptr.is_null() {
                let shared = &*ptr;
                shared.lock().map(|s| s.drag_mode).unwrap_or(false)
            } else {
                false
            };

            if !in_drag_mode {
                return LRESULT(HTTRANSPARENT as isize);
            }

            // In drag mode: allow dragging anywhere on window, or resizing by corners/edges!
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;

            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);

            let border_edge = 10;
            let corner = 22;

            let is_br = x >= rect.right - corner && y >= rect.bottom - corner;
            let is_bl = x <= rect.left + corner && y >= rect.bottom - corner;
            let is_tr = x >= rect.right - corner && y <= rect.top + corner;
            let is_tl = x <= rect.left + corner && y <= rect.top + corner;

            if is_br {
                LRESULT(HTBOTTOMRIGHT as isize)
            } else if is_bl {
                LRESULT(HTBOTTOMLEFT as isize)
            } else if is_tr {
                LRESULT(HTTOPRIGHT as isize)
            } else if is_tl {
                LRESULT(HTTOPLEFT as isize)
            } else if x >= rect.right - border_edge {
                LRESULT(HTRIGHT as isize)
            } else if x <= rect.left + border_edge {
                LRESULT(HTLEFT as isize)
            } else if y >= rect.bottom - border_edge {
                LRESULT(HTBOTTOM as isize)
            } else if y <= rect.top + border_edge {
                LRESULT(HTTOP as isize)
            } else {
                // HTCAPTION enables seamless, native Windows window dragging!
                LRESULT(HTCAPTION as isize)
            }
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            // Fill window with COLOR_KEY (pure black), which is 100% transparent.
            let bg_brush = CreateSolidBrush(COLOR_KEY);
            let _ = FillRect(hdc, &client_rect, bg_brush);
            let _ = DeleteObject(bg_brush.into());

            // Retrieve text, drag_mode, font_size & theme
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<HudSharedState>;
            let (text, drag_mode, font_size, theme) = if !ptr.is_null() {
                let shared = &*ptr;
                shared.lock().map(|s| (s.text.clone(), s.drag_mode, s.font_size, s.theme)).unwrap_or_default()
            } else {
                (String::new(), false, 22, HudTheme::GhostWhite)
            };

            // If in drag mode: render visible repositioning/resizing box outline and guide
            if drag_mode {
                let border_brush = CreateSolidBrush(COLORREF(0x00FFD700)); // Gold accent
                let _ = FrameRect(hdc, &client_rect, border_brush);
                let _ = DeleteObject(border_brush.into());

                // Bottom-right resize grip
                let grip_rect = RECT {
                    left: client_rect.right - 14,
                    top: client_rect.bottom - 14,
                    right: client_rect.right - 2,
                    bottom: client_rect.bottom - 2,
                };
                let grip_brush = CreateSolidBrush(COLORREF(0x00FFD700));
                let _ = FillRect(hdc, &grip_rect, grip_brush);
                let _ = DeleteObject(grip_brush.into());

                // Guide banner
                let guide_font = CreateFontW(
                    13, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
                    DEFAULT_CHARSET, OUT_TT_PRECIS,
                    windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                    CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
                );
                let old_font = SelectObject(hdc, guide_font.into());
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, COLORREF(0x00FFD700));
                let mut guide_text: Vec<u16> = "✥ DRAG TO MOVE | DRAG CORNERS TO RESIZE | CLICK 'LOCK' IN MENU WHEN DONE"
                    .encode_utf16().collect();
                let mut guide_rect = RECT {
                    left: client_rect.left + 8,
                    top: client_rect.top + 4,
                    right: client_rect.right - 8,
                    bottom: client_rect.top + 20,
                };
                let _ = DrawTextW(hdc, &mut guide_text, &raw mut guide_rect, DT_CENTER | DT_VCENTER);
                let _ = SelectObject(hdc, old_font);
                let _ = DeleteObject(guide_font.into());
            }

            if !text.is_empty() {
                // Scalable font based on user setting
                let font = CreateFontW(
                    font_size,
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

                let mut characters: Vec<u16> = text.encode_utf16().collect();
                let top_offset = if drag_mode { 22 } else { 8 };

                // 1. Draw 8-way subtle dark outline/halo
                let _ = SetTextColor(hdc, theme.shadow_color());
                let offsets = [
                    (-1, -1), (0, -1), (1, -1),
                    (-1,  0),          (1,  0),
                    (-1,  1), (0,  1), (1,  1),
                    (2, 2),
                ];
                for (dx, dy) in offsets {
                    let mut shadow_rect = RECT {
                        left: client_rect.left + 16 + dx,
                        top: client_rect.top + top_offset + dy,
                        right: client_rect.right - 16 + dx,
                        bottom: client_rect.bottom - 8 + dy,
                    };
                    let mut shadow_chars = characters.clone();
                    let _ = DrawTextW(
                        hdc,
                        &mut shadow_chars,
                        &raw mut shadow_rect,
                        DT_CENTER | DT_WORDBREAK,
                    );
                }

                // 2. Draw crisp foreground text in chosen theme color
                let _ = SetTextColor(hdc, theme.text_color());
                let mut main_rect = RECT {
                    left: client_rect.left + 16,
                    top: client_rect.top + top_offset,
                    right: client_rect.right - 16,
                    bottom: client_rect.bottom - 8,
                };
                let _ = DrawTextW(
                    hdc,
                    &mut characters,
                    &raw mut main_rect,
                    DT_CENTER | DT_WORDBREAK,
                );

                let _ = SelectObject(hdc, old_font);
                let _ = DeleteObject(font.into());
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let pointer = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut Mutex<HudSharedState>;
            if !pointer.is_null() {
                drop(Arc::from_raw(pointer));
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
