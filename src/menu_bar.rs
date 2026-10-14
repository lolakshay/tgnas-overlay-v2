use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
            FillRect, FrameRect, InvalidateRect, SelectObject, SetBkMode, SetTextColor,
            CLEARTYPE_QUALITY, DEFAULT_CHARSET, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FW_BOLD,
            FW_SEMIBOLD, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

const CLASS_NAME: PCWSTR = w!("System.Dwm.QuickControl");

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ButtonId {
    Solve,
    ToggleAnswers,
    AskCustomQuestion,
    CyclePromptMode,
    OpacityDown,
    OpacityUp,
    FontDown,
    FontUp,
    CycleTheme,
    DragArea,
    ToggleClickThrough,
    PrevHistory,
    NextHistory,
    ExportSession,
    ToggleVpn,
    Close,
}

pub struct MenuBarState {
    pub main_thread_id: u32,
    pub opacity: u8,
    pub font_size: i32,
    pub theme_name: String,
    pub drag_mode_active: bool,
    pub history_current: usize,
    pub history_total: usize,
    pub vpn_enabled: bool,
    pub prompt_mode: crate::config::PromptMode,
    pub click_through: bool,
    pub hovered_btn: Option<ButtonId>,
}

pub struct MenuBar {
    hwnd: HWND,
    visible: bool,
    state: Arc<Mutex<MenuBarState>>,
}

impl MenuBar {
    pub unsafe fn create(
        initially_visible: bool,
        opacity: u8,
        font_size: i32,
        theme_name: &str,
        vpn_enabled: bool,
        prompt_mode: crate::config::PromptMode,
        main_thread_id: u32,
        custom_pos: Option<(i32, i32)>,
    ) -> Result<Self> {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(wndproc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 && windows::core::Error::from_thread().code().0 != 1410 {
            return Err(anyhow!("RegisterClassExW for MenuBar failed"));
        }

        let mut work_area = RECT::default();
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&raw mut work_area).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;

        let bar_w = 320;
        let bar_h = 420;
        let (bar_x, bar_y) = match custom_pos {
            Some((x, y)) if x >= 0 && y >= 0 => (x, y),
            _ => (work_area.left + 24, work_area.top + 24),
        };

        let state = Arc::new(Mutex::new(MenuBarState {
            main_thread_id,
            opacity,
            font_size,
            theme_name: theme_name.to_string(),
            drag_mode_active: false,
            history_current: 0,
            history_total: 0,
            vpn_enabled,
            prompt_mode,
            click_through: false,
            hovered_btn: None,
        }));
        let raw_state = Arc::into_raw(state.clone());

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            CLASS_NAME,
            w!("Master Control"),
            WS_POPUP,
            bar_x,
            bar_y,
            bar_w,
            bar_h,
            None,
            None,
            Some(instance),
            Some(raw_state as *const std::ffi::c_void),
        )?;

        // Apply adjustable opacity directly to the Menu Bar window
        SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA)
            .ok()
            .ok_or_else(|| anyhow!("SetLayeredWindowAttributes failed for MenuBar"))?;

        // Exclude from screenshots and recordings
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);

        let show_flag = if initially_visible {
            SWP_SHOWWINDOW
        } else {
            SWP_HIDEWINDOW
        };
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            bar_x,
            bar_y,
            bar_w,
            bar_h,
            SWP_NOACTIVATE | show_flag,
        )
        .ok()
        .ok_or_else(|| anyhow!("SetWindowPos failed for MenuBar"))?;

        Ok(Self {
            hwnd,
            visible: initially_visible,
            state,
        })
    }

    pub fn set_visible(&mut self, visible: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
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

    pub fn get_pos(&self) -> (i32, i32) {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut rect);
            (rect.left, rect.top)
        }
    }

    pub fn update_history(&self, current: usize, total: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.history_current = current;
            state.history_total = total;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn update_opacity(&self, opacity: u8) {
        if let Ok(mut state) = self.state.lock() {
            state.opacity = opacity;
        }
        unsafe {
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), opacity, LWA_ALPHA);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn update_font_size(&self, font_size: i32) {
        if let Ok(mut state) = self.state.lock() {
            state.font_size = font_size;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn update_theme(&self, theme_name: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.theme_name = theme_name.to_string();
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn set_drag_mode(&self, active: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.drag_mode_active = active;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn update_vpn(&self, enabled: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.vpn_enabled = enabled;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn update_prompt_mode(&self, mode: crate::config::PromptMode) {
        if let Ok(mut state) = self.state.lock() {
            state.prompt_mode = mode;
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn set_click_through(&self, click_through: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.click_through = click_through;
        }
        unsafe {
            let ex_style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(self.hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
            let new_style = if click_through {
                ex_style | (windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT.0 as isize)
            } else {
                ex_style & !(windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT.0 as isize)
            };
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(self.hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE, new_style);
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED,
            );
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn is_click_through(&self) -> bool {
        self.state.lock().map(|s| s.click_through).unwrap_or(false)
    }

    pub fn toggle_click_through(&self) -> bool {
        let current = self.is_click_through();
        let new_val = !current;
        self.set_click_through(new_val);
        new_val
    }
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn get_button_rects(w: i32, _h: i32) -> Vec<(ButtonId, RECT)> {
    vec![
        // Close button on top-right of header banner
        (ButtonId::Close, RECT { left: w - 30, top: 6, right: w - 8, bottom: 28 }),
        // Row 1: Actions (Solve & Toggle Answers side-by-side)
        (ButtonId::Solve, RECT { left: 8, top: 36, right: 156, bottom: 64 }),
        (ButtonId::ToggleAnswers, RECT { left: 164, top: 36, right: w - 8, bottom: 64 }),
        // Row 2: Ask Custom Question (Full width button)
        (ButtonId::AskCustomQuestion, RECT { left: 8, top: 68, right: w - 8, bottom: 96 }),
        // Row 3: Prompt Mode Switcher (Full width carousel)
        (ButtonId::CyclePromptMode, RECT { left: 8, top: 100, right: w - 8, bottom: 128 }),
        // Row 4: Opacity Adjustment
        (ButtonId::OpacityDown, RECT { left: 8, top: 132, right: 44, bottom: 160 }),
        (ButtonId::OpacityUp, RECT { left: w - 44, top: 132, right: w - 8, bottom: 160 }),
        // Row 5: Font Size Adjustment
        (ButtonId::FontDown, RECT { left: 8, top: 164, right: 44, bottom: 192 }),
        (ButtonId::FontUp, RECT { left: w - 44, top: 164, right: w - 8, bottom: 192 }),
        // Row 6: Theme Presets
        (ButtonId::CycleTheme, RECT { left: 8, top: 196, right: w - 8, bottom: 224 }),
        // Row 7: Reposition Text Area
        (ButtonId::DragArea, RECT { left: 8, top: 228, right: w - 8, bottom: 256 }),
        // Row 8: Menu Click-Through (Ghost Mode)
        (ButtonId::ToggleClickThrough, RECT { left: 8, top: 260, right: w - 8, bottom: 288 }),
        // Row 9: History Navigation
        (ButtonId::PrevHistory, RECT { left: 8, top: 292, right: 44, bottom: 320 }),
        (ButtonId::NextHistory, RECT { left: w - 44, top: 292, right: w - 8, bottom: 320 }),
        // Row 10: Session Export
        (ButtonId::ExportSession, RECT { left: 8, top: 324, right: w - 8, bottom: 352 }),
        // Row 11: Stealth VPN Toggle
        (ButtonId::ToggleVpn, RECT { left: 8, top: 356, right: w - 8, bottom: 384 }),
    ]
}

fn hit_test_button(x: i32, y: i32, w: i32, h: i32) -> Option<ButtonId> {
    for (id, r) in get_button_rects(w, h) {
        if x >= r.left && x <= r.right && y >= r.top && y <= r.bottom {
            return Some(id);
        }
    }
    None
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<MenuBarState>;
            if !ptr.is_null() {
                let shared = &*ptr;
                let main_thread_id = shared.lock().map(|s| s.main_thread_id).unwrap_or(0);
                let _ = PostThreadMessageW(
                    main_thread_id,
                    crate::hotkeys::WM_APP_SAVE_MENU_POS,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_NCHITTEST => {
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            let pt_x = (lp.0 & 0xFFFF) as i16 as i32;
            let pt_y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;

            let mut win_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut win_rect);
            let local_x = pt_x - win_rect.left;
            let local_y = pt_y - win_rect.top;

            if hit_test_button(local_x, local_y, client_rect.right, client_rect.bottom).is_some() {
                LRESULT(HTCLIENT as isize)
            } else {
                // Clicking & dragging any non-button area moves the entire menu bar smoothly!
                LRESULT(HTCAPTION as isize)
            }
        }
        WM_SETCURSOR => {
            let mut pt = windows::Win32::Foundation::POINT::default();
            let _ = GetCursorPos(&mut pt);
            let mut win_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut win_rect);
            let local_x = pt.x - win_rect.left;
            let local_y = pt.y - win_rect.top;

            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            if hit_test_button(local_x, local_y, client_rect.right, client_rect.bottom).is_some() {
                let _ = SetCursor(Some(LoadCursorW(None, IDC_HAND).unwrap_or_default()));
                return LRESULT(1);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_MOUSEMOVE => {
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let btn = hit_test_button(x, y, client_rect.right, client_rect.bottom);

            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<MenuBarState>;
            if !ptr.is_null() {
                let shared = &*ptr;
                if let Ok(mut state) = shared.lock() {
                    if state.hovered_btn != btn {
                        state.hovered_btn = btn;
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;

            if let Some(btn) = hit_test_button(x, y, client_rect.right, client_rect.bottom) {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<MenuBarState>;
                if !ptr.is_null() {
                    let shared = &*ptr;
                    let main_thread_id = shared.lock().map(|s| s.main_thread_id).unwrap_or(0);
                    match btn {
                        ButtonId::Solve => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_SCREENSHOT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::ToggleAnswers => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_TOGGLE_ANSWERS,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::AskCustomQuestion => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_OPEN_CUSTOM_QUESTION,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::CyclePromptMode => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_CYCLE_PROMPT_MODE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::OpacityDown => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_OPACITY_CHANGE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::OpacityUp => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_OPACITY_CHANGE,
                                windows::Win32::Foundation::WPARAM(1),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::FontDown => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_FONT_SIZE_CHANGE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::FontUp => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_FONT_SIZE_CHANGE,
                                windows::Win32::Foundation::WPARAM(1),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::CycleTheme => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_CYCLE_THEME,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::DragArea => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_TOGGLE_DRAG_MODE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::ToggleClickThrough => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_TOGGLE_MENU_CLICK_THROUGH,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::PrevHistory => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_PREV_HISTORY,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::NextHistory => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_NEXT_HISTORY,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::ExportSession => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_EXPORT_SESSION,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::ToggleVpn => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_TOGGLE_VPN,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        ButtonId::Close => {
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_QUIT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            // Read state
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<MenuBarState>;
            let (opacity, font_size, theme_name, drag_mode, cur_hist, tot_hist, vpn_enabled, prompt_mode, click_through, hovered) = if !ptr.is_null() {
                let shared = &*ptr;
                shared.lock().map(|s| (
                    s.opacity,
                    s.font_size,
                    s.theme_name.clone(),
                    s.drag_mode_active,
                    s.history_current,
                    s.history_total,
                    s.vpn_enabled,
                    s.prompt_mode,
                    s.click_through,
                    s.hovered_btn,
                )).unwrap_or((180, 22, "White".into(), false, 0, 0, false, crate::config::PromptMode::Mcq, false, None))
            } else {
                (180, 22, "White".into(), false, 0, 0, false, crate::config::PromptMode::Mcq, false, None)
            };

            // 1. Draw sleek frosted dark bar background (RGB 22, 22, 28 -> 0x001C1616)
            let bg_brush = CreateSolidBrush(COLORREF(0x001E1A18));
            let _ = FillRect(hdc, &client_rect, bg_brush);
            let _ = DeleteObject(bg_brush.into());

            // 1b. GTA 5 Mod Menu Top Accent Stripe: Glowing Gold/Amber
            let top_stripe = RECT { left: 0, top: 0, right: client_rect.right, bottom: 2 };
            let stripe_brush = CreateSolidBrush(COLORREF(0x0000D7FF));
            let _ = FillRect(hdc, &top_stripe, stripe_brush);
            let _ = DeleteObject(stripe_brush.into());

            // 2. Draw subtle border (Glowing Neon Magenta/Purple when Click-Through is ON!)
            let border_brush = if click_through {
                CreateSolidBrush(COLORREF(0x00FF33CC))
            } else {
                CreateSolidBrush(COLORREF(0x003A3430))
            };
            let _ = FrameRect(hdc, &client_rect, border_brush);
            let _ = DeleteObject(border_brush.into());

            let font = CreateFontW(
                13, 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_TT_PRECIS,
                windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
            );
            let bold_font = CreateFontW(
                13, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_TT_PRECIS,
                windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, font.into());
            let _ = SetBkMode(hdc, TRANSPARENT);

            // 3. Draw Header Title: "⚡ CONTROL MENU" (or GHOST MENU)
            let (title, title_color) = if click_through {
                ("⚡ GHOST MENU", COLORREF(0x00FF80FF))
            } else {
                ("⚡ CONTROL MENU", COLORREF(0x0000D7FF))
            };
            let _ = SetTextColor(hdc, title_color);
            let mut title_chars: Vec<u16> = title.encode_utf16().collect();
            let mut title_rect = RECT { left: 12, top: 4, right: client_rect.right - 34, bottom: 30 };
            let _ = SelectObject(hdc, bold_font.into());
            let _ = DrawTextW(hdc, &mut title_chars, &raw mut title_rect, DT_VCENTER | DT_SINGLELINE);
            let _ = SelectObject(hdc, font.into());

            // Header separator line
            let sep_rect = RECT { left: 8, top: 32, right: client_rect.right - 8, bottom: 33 };
            let sep_brush = CreateSolidBrush(COLORREF(0x003A3430));
            let _ = FillRect(hdc, &sep_rect, sep_brush);
            let _ = DeleteObject(sep_brush.into());

            // 4. Draw interactive buttons
            let buttons = get_button_rects(client_rect.right, client_rect.bottom);
            for (id, r) in buttons {
                let is_hovered = hovered == Some(id);

                // Button background color
                let btn_bg_color = match id {
                    ButtonId::AskCustomQuestion => {
                        if is_hovered { COLORREF(0x003A2E14) } else { COLORREF(0x00261C10) }
                    }
                    ButtonId::CyclePromptMode => {
                        if is_hovered { COLORREF(0x00362818) } else { COLORREF(0x00261C14) }
                    }
                    ButtonId::DragArea if drag_mode => {
                        if is_hovered { COLORREF(0x002E8040) } else { COLORREF(0x00246B34) }
                    }
                    ButtonId::ToggleClickThrough => {
                        if click_through {
                            if is_hovered { COLORREF(0x004A1A4A) } else { COLORREF(0x00361236) }
                        } else {
                            if is_hovered { COLORREF(0x003A3228) } else { COLORREF(0x00262018) }
                        }
                    }
                    ButtonId::ExportSession => {
                        if is_hovered { COLORREF(0x005A3A1E) } else { COLORREF(0x00422A14) }
                    }
                    ButtonId::ToggleVpn => {
                        if vpn_enabled {
                            if is_hovered { COLORREF(0x00404018) } else { COLORREF(0x002A3010) }
                        } else {
                            if is_hovered { COLORREF(0x00403834) } else { COLORREF(0x002C2624) }
                        }
                    }
                    ButtonId::Close => {
                        if is_hovered { COLORREF(0x002A20A0) } else { COLORREF(0x002C2624) }
                    }
                    _ => {
                        if is_hovered { COLORREF(0x00403834) } else { COLORREF(0x002C2624) }
                    }
                };

                let btn_brush = CreateSolidBrush(btn_bg_color);
                let _ = FillRect(hdc, &r, btn_brush);
                let _ = DeleteObject(btn_brush.into());

                // Button border
                let border_color = if id == ButtonId::AskCustomQuestion {
                    if is_hovered { COLORREF(0x0000FFFF) } else { COLORREF(0x0000D7FF) }
                } else if id == ButtonId::CyclePromptMode {
                    match prompt_mode {
                        crate::config::PromptMode::Mcq => COLORREF(0x0000D7FF), // GTA Gold
                        crate::config::PromptMode::Code => COLORREF(0x0050FF50), // Matrix Green
                        crate::config::PromptMode::Math => COLORREF(0x00FFFF00), // Cyber Cyan
                        crate::config::PromptMode::TrueFalse => COLORREF(0x00FF66CC), // Neon Magenta
                        crate::config::PromptMode::Custom => COLORREF(0x00FF80FF), // Neon Purple/Pink
                    }
                } else if id == ButtonId::DragArea && drag_mode {
                    COLORREF(0x0000FF80) // Bright green when active
                } else if id == ButtonId::ToggleClickThrough && click_through {
                    COLORREF(0x00FF33CC) // Neon Magenta when ghost click-through is active
                } else if id == ButtonId::ExportSession {
                    COLORREF(0x0080A030)
                } else if id == ButtonId::ToggleVpn && vpn_enabled {
                    COLORREF(0x0000FFCC) // Bright cyan/green glow when VPN active
                } else if id == ButtonId::Close && is_hovered {
                    COLORREF(0x004444FF) // Red glow when closing
                } else if is_hovered {
                    COLORREF(0x00706058)
                } else {
                    COLORREF(0x00443C36)
                };
                let b_brush = CreateSolidBrush(border_color);
                let _ = FrameRect(hdc, &r, b_brush);
                let _ = DeleteObject(b_brush.into());

                // Button label & text color
                let theme_btn_label = format!("🎨 Theme: {}", theme_name);
                let prompt_mode_label = match prompt_mode {
                    crate::config::PromptMode::Mcq => "🎯 Mode: < MCQ Only >".to_string(),
                    crate::config::PromptMode::Code => "🎯 Mode: < Code Snippets >".to_string(),
                    crate::config::PromptMode::Math => "🎯 Mode: < Math & Steps >".to_string(),
                    crate::config::PromptMode::TrueFalse => "🎯 Mode: < True/False & Fill >".to_string(),
                    crate::config::PromptMode::Custom => "🎯 Mode: < Custom Q&A >".to_string(),
                };
                let (label, text_color): (&str, COLORREF) = match id {
                    ButtonId::Solve => ("📸 Solve (P+O)", COLORREF(0x00FFFFFF)),
                    ButtonId::ToggleAnswers => ("👁 Answer (L+K)", COLORREF(0x00E0E0E0)),
                    ButtonId::AskCustomQuestion => ("💬 Ask Custom Question (I+O)", COLORREF(0x0000D7FF)),
                    ButtonId::CyclePromptMode => {
                        let color = match prompt_mode {
                            crate::config::PromptMode::Mcq => COLORREF(0x0000D7FF),
                            crate::config::PromptMode::Code => COLORREF(0x0050FF50),
                            crate::config::PromptMode::Math => COLORREF(0x00FFFF00),
                            crate::config::PromptMode::TrueFalse => COLORREF(0x00FF66CC),
                            crate::config::PromptMode::Custom => COLORREF(0x00FF80FF),
                        };
                        (&prompt_mode_label, color)
                    }
                    ButtonId::OpacityDown => ("-", COLORREF(0x00E0E0E0)),
                    ButtonId::OpacityUp => ("+", COLORREF(0x00E0E0E0)),
                    ButtonId::FontDown => ("-", COLORREF(0x00E0E0E0)),
                    ButtonId::FontUp => ("+", COLORREF(0x00E0E0E0)),
                    ButtonId::CycleTheme => (&theme_btn_label, COLORREF(0x00E0FFFF)),
                    ButtonId::DragArea => {
                        if drag_mode {
                            ("🔒 Lock Caption Area", COLORREF(0x00FFFFFF))
                        } else {
                            ("✥ Move Caption Area", COLORREF(0x00FFD700))
                        }
                    }
                    ButtonId::ToggleClickThrough => {
                        if click_through {
                            ("👻 Click-Through: ON (Ghost)", COLORREF(0x00FF80FF))
                        } else {
                            ("👻 Menu Click-Through: OFF", COLORREF(0x00C0C0C0))
                        }
                    }
                    ButtonId::PrevHistory => ("◀", COLORREF(0x00FFFFFF)),
                    ButtonId::NextHistory => ("▶", COLORREF(0x00FFFFFF)),
                    ButtonId::ExportSession => ("💾 Save Session (.md / .txt)", COLORREF(0x0066FFCC)),
                    ButtonId::ToggleVpn => {
                        if vpn_enabled {
                            ("🛡️ Stealth VPN: ON", COLORREF(0x0066FFFF))
                        } else {
                            ("🛡️ Stealth VPN: OFF", COLORREF(0x00A0A0A0))
                        }
                    }
                    ButtonId::Close => ("✕", if is_hovered { COLORREF(0x00FFFFFF) } else { COLORREF(0x00A0A0A0) }),
                };

                let _ = SetTextColor(hdc, text_color);
                let mut mut_rect = r;
                let mut chars: Vec<u16> = label.encode_utf16().collect();
                let _ = DrawTextW(hdc, &mut chars, &raw mut mut_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }

            // 5. Draw inset slider readout displays
            let inset_bg = CreateSolidBrush(COLORREF(0x0014100E));
            let inset_border = CreateSolidBrush(COLORREF(0x00332B26));

            // Opacity readout (Row 4)
            let mut op_rect = RECT { left: 48, top: 132, right: client_rect.right - 48, bottom: 160 };
            let _ = FillRect(hdc, &op_rect, inset_bg);
            let _ = FrameRect(hdc, &op_rect, inset_border);
            let _ = SetTextColor(hdc, COLORREF(0x00FFFFFF));
            let pct = (opacity as f32 / 255.0 * 100.0).round() as u32;
            let mut op_val: Vec<u16> = format!("Opacity:  {}%", pct).encode_utf16().collect();
            let _ = DrawTextW(hdc, &mut op_val, &raw mut op_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Font size readout (Row 5)
            let mut font_rect = RECT { left: 48, top: 164, right: client_rect.right - 48, bottom: 192 };
            let _ = FillRect(hdc, &font_rect, inset_bg);
            let _ = FrameRect(hdc, &font_rect, inset_border);
            let mut font_val: Vec<u16> = format!("Font Size:  {}px", font_size).encode_utf16().collect();
            let _ = DrawTextW(hdc, &mut font_val, &raw mut font_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // History count readout (Row 9)
            let mut hist_rect = RECT { left: 48, top: 292, right: client_rect.right - 48, bottom: 320 };
            let _ = FillRect(hdc, &hist_rect, inset_bg);
            let _ = FrameRect(hdc, &hist_rect, inset_border);
            let hist_display = if tot_hist == 0 {
                "History:  Empty".to_string()
            } else {
                format!("History:  {} / {}", cur_hist, tot_hist)
            };
            let mut hist_val: Vec<u16> = hist_display.encode_utf16().collect();
            let _ = DrawTextW(hdc, &mut hist_val, &raw mut hist_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            let _ = DeleteObject(inset_bg.into());
            let _ = DeleteObject(inset_border.into());

            // 6. Draw Footer Status / Instructions (Below Row 11)
            let _ = SetTextColor(hdc, COLORREF(0x00787068));
            let mut footer_rect = RECT { left: 8, top: 388, right: client_rect.right - 8, bottom: 410 };
            let mut footer_text: Vec<u16> = "⌨ P+O Solve | I+O Ask | M+N Menu".encode_utf16().collect();
            let _ = DrawTextW(hdc, &mut footer_text, &raw mut footer_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(font.into());
            let _ = DeleteObject(bold_font.into());

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let pointer = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut Mutex<MenuBarState>;
            if !pointer.is_null() {
                drop(Arc::from_raw(pointer));
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
