use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
            FillRect, FrameRect, InvalidateRect, SelectObject, SetBkColor, SetBkMode, SetTextColor,
            CLEARTYPE_QUALITY, DEFAULT_CHARSET, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FW_BOLD,
            FW_SEMIBOLD, HDC, HFONT, HGDIOBJ, OUT_TT_PRECIS, PAINTSTRUCT, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::*},
    },
};

const CLASS_NAME: PCWSTR = w!("TransparentOverlay.CustomQuestionDialog");
const EM_SETSEL: u32 = 0x00B1;

const ID_QUESTION_INPUT: i32 = 1001;
const ID_PRESET_EXPLAIN: i32 = 1002;
const ID_PRESET_CODE: i32 = 1003;
const ID_PRESET_MCQ: i32 = 1004;
const ID_PRESET_STEPS: i32 = 1005;
const ID_CHECK_CAPTURE_NEW: i32 = 1006;
const ID_CHECK_SET_DEFAULT: i32 = 1007;
const ID_SUBMIT: i32 = 1008;
const ID_CANCEL: i32 = 1009;

#[derive(Clone, Debug)]
pub struct CustomQuestionSubmission {
    pub question: String,
    pub capture_new: bool,
    pub set_as_default: bool,
}

struct DialogInner {
    #[allow(dead_code)]
    hwnd: HWND,
    edit_hwnd: HWND,
    chk_capture_hwnd: HWND,
    chk_default_hwnd: HWND,
    tx: Sender<CustomQuestionSubmission>,
    main_thread_id: u32,
    opacity: AtomicU8,
    bg_brush: CreateSolidBrushHandle,
    edit_bg_brush: CreateSolidBrushHandle,
    _font: HFONTHandle,
}

// RAII wrapper for HBRUSH
struct CreateSolidBrushHandle(windows::Win32::Graphics::Gdi::HBRUSH);
impl Drop for CreateSolidBrushHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}

// RAII wrapper for HFONT
struct HFONTHandle(HFONT);
impl Drop for HFONTHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}

pub struct CustomQuestionDialog {
    hwnd: HWND,
    inner: Arc<DialogInner>,
}

unsafe extern "system" fn edit_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    let orig = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const ();
    if msg == WM_KEYDOWN {
        if wp.0 == 0x0D {
            // VK_RETURN pressed
            let parent = GetParent(hwnd).unwrap_or_default();
            let _ = PostMessageW(Some(parent), WM_COMMAND, WPARAM(ID_SUBMIT as usize), LPARAM(0));
            return LRESULT(0);
        } else if wp.0 == 0x1B {
            // VK_ESCAPE pressed
            let parent = GetParent(hwnd).unwrap_or_default();
            let _ = PostMessageW(Some(parent), WM_COMMAND, WPARAM(ID_CANCEL as usize), LPARAM(0));
            return LRESULT(0);
        }
    }
    if !orig.is_null() {
        CallWindowProcW(Some(std::mem::transmute(orig)), hwnd, msg, wp, lp)
    } else {
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

impl CustomQuestionDialog {
    pub unsafe fn create(
        main_thread_id: u32,
        tx: Sender<CustomQuestionSubmission>,
        anchor_pos: Option<(i32, i32)>,
        opacity: u8,
    ) -> Result<Self> {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(dialog_wndproc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 && windows::core::Error::from_thread().code().0 != 1410 {
            return Err(anyhow!("RegisterClassExW for CustomQuestionDialog failed"));
        }

        let mut work_area = RECT::default();
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&raw mut work_area).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;

        let win_w = 480;
        let win_h = 224;

        let (win_x, win_y) = calculate_dialog_pos(anchor_pos, win_w, win_h, &work_area);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS_NAME,
            w!("Ask Custom Question"),
            WS_POPUP,
            win_x,
            win_y,
            win_w,
            win_h,
            None,
            None,
            Some(instance),
            None,
        )?;

        // Apply same opacity configured for the app
        SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA)?;
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);

        let bg_brush = CreateSolidBrush(COLORREF(0x001E1A18));
        let edit_bg_brush = CreateSolidBrush(COLORREF(0x0014100E));

        // Create UI Controls with pixel-perfect alignment
        let edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            16,
            54,
            win_w - 32,
            26,
            Some(hwnd),
            Some(HMENU(ID_QUESTION_INPUT as usize as *mut _)),
            Some(instance),
            None,
        )?;

        let orig_proc = SetWindowLongPtrW(
            edit_hwnd,
            GWLP_WNDPROC,
            edit_subclass_proc as *const () as usize as isize,
        );
        SetWindowLongPtrW(edit_hwnd, GWLP_USERDATA, orig_proc);

        // Preset Chips (Row at y: 88, 4 equal chips spanning 16..464)
        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("⚡ Explain"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            16,
            88,
            106,
            24,
            Some(hwnd),
            Some(HMENU(ID_PRESET_EXPLAIN as usize as *mut _)),
            Some(instance),
            None,
        )?;

        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("💻 Code"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            130,
            88,
            106,
            24,
            Some(hwnd),
            Some(HMENU(ID_PRESET_CODE as usize as *mut _)),
            Some(instance),
            None,
        )?;

        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("🔍 Solve MCQ"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            244,
            88,
            106,
            24,
            Some(hwnd),
            Some(HMENU(ID_PRESET_MCQ as usize as *mut _)),
            Some(instance),
            None,
        )?;

        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("📝 Steps"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            358,
            88,
            106,
            24,
            Some(hwnd),
            Some(HMENU(ID_PRESET_STEPS as usize as *mut _)),
            Some(instance),
            None,
        )?;

        // Checkboxes (Row at y: 122)
        let chk_capture_hwnd = CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("Capture fresh screenshot"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            16,
            122,
            210,
            22,
            Some(hwnd),
            Some(HMENU(ID_CHECK_CAPTURE_NEW as usize as *mut _)),
            Some(instance),
            None,
        )?;
        let _ = SendMessageW(chk_capture_hwnd, BM_SETCHECK, Some(WPARAM(1)), Some(LPARAM(0)));

        let chk_default_hwnd = CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("Set as active Custom Mode prompt"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            236,
            122,
            228,
            22,
            Some(hwnd),
            Some(HMENU(ID_CHECK_SET_DEFAULT as usize as *mut _)),
            Some(instance),
            None,
        )?;

        // Action Buttons (Row at y: 154)
        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("🚀 Ask AI & Solve (Enter)"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            16,
            154,
            300,
            32,
            Some(hwnd),
            Some(HMENU(ID_SUBMIT as usize as *mut _)),
            Some(instance),
            None,
        )?;

        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("Cancel (Esc)"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            326,
            154,
            138,
            32,
            Some(hwnd),
            Some(HMENU(ID_CANCEL as usize as *mut _)),
            Some(instance),
            None,
        )?;

        let font = CreateFontW(
            13, 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0,
            DEFAULT_CHARSET, OUT_TT_PRECIS,
            windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
            CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
        );

        // Apply font to all child controls
        for child_id in [
            ID_QUESTION_INPUT, ID_PRESET_EXPLAIN, ID_PRESET_CODE, ID_PRESET_MCQ, ID_PRESET_STEPS,
            ID_CHECK_CAPTURE_NEW, ID_CHECK_SET_DEFAULT, ID_SUBMIT, ID_CANCEL
        ] {
            if let Ok(c) = GetDlgItem(Some(hwnd), child_id) {
                let _ = SendMessageW(c, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
            }
        }

        let inner = Arc::new(DialogInner {
            hwnd,
            edit_hwnd,
            chk_capture_hwnd,
            chk_default_hwnd,
            tx,
            main_thread_id,
            opacity: AtomicU8::new(opacity),
            bg_brush: CreateSolidBrushHandle(bg_brush),
            edit_bg_brush: CreateSolidBrushHandle(edit_bg_brush),
            _font: HFONTHandle(font),
        });

        let raw_inner = Arc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_inner as isize);

        Ok(Self { hwnd, inner })
    }

    pub fn update_opacity(&self, opacity: u8) {
        self.inner.opacity.store(opacity, Ordering::Relaxed);
        unsafe {
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), opacity, LWA_ALPHA);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn show(&self, current_custom_prompt: &str, anchor_pos: Option<(i32, i32)>) {
        unsafe {
            let mut work_area = RECT::default();
            let _ = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some((&raw mut work_area).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );

            let win_w = 480;
            let win_h = 224;
            let (target_x, target_y) = calculate_dialog_pos(anchor_pos, win_w, win_h, &work_area);

            // Re-apply current opacity
            let cur_op = self.inner.opacity.load(Ordering::Relaxed);
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), cur_op, LWA_ALPHA);

            let len = GetWindowTextLengthW(self.inner.edit_hwnd);
            if len == 0 && !current_custom_prompt.trim().is_empty() {
                let hstr = HSTRING::from(current_custom_prompt);
                let _ = SetWindowTextW(self.inner.edit_hwnd, PCWSTR(hstr.as_ptr()));
            }

            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                target_x,
                target_y,
                win_w,
                win_h,
                SWP_SHOWWINDOW,
            );
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(Some(self.inner.edit_hwnd));
            let _ = SendMessageW(self.inner.edit_hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }
}

fn calculate_dialog_pos(anchor_pos: Option<(i32, i32)>, win_w: i32, win_h: i32, work_area: &RECT) -> (i32, i32) {
    match anchor_pos {
        Some((mx, my)) => {
            // Place 8px to the right of the Menu Bar (width: 320)
            let mut target_x = mx + 328;
            if target_x + win_w > work_area.right - 10 {
                // If overflows right screen edge, place to the left of Menu Bar
                target_x = mx - win_w - 8;
            }
            let clamped_x = target_x.max(work_area.left + 10).min(work_area.right - win_w - 10);
            let clamped_y = my.max(work_area.top + 10).min(work_area.bottom - win_h - 10);
            (clamped_x, clamped_y)
        }
        None => {
            let center_x = (work_area.right - work_area.left - win_w) / 2;
            let center_y = (work_area.bottom - work_area.top - win_h) / 2;
            (center_x, center_y)
        }
    }
}

impl Drop for CustomQuestionDialog {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn dialog_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CTLCOLORSTATIC => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DialogInner;
            if !ptr.is_null() {
                let inner = &*ptr;
                let hdc = HDC(wp.0 as *mut _);
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, COLORREF(0x00E0D8D0));
                return LRESULT(inner.bg_brush.0 .0 as isize);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_CTLCOLOREDIT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DialogInner;
            if !ptr.is_null() {
                let inner = &*ptr;
                let hdc = HDC(wp.0 as *mut _);
                let _ = SetBkColor(hdc, COLORREF(0x0014100E));
                let _ = SetTextColor(hdc, COLORREF(0x00FFFFFF));
                return LRESULT(inner.edit_bg_brush.0 .0 as isize);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_COMMAND => {
            let cmd_id = (wp.0 & 0xFFFF) as i32;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DialogInner;
            if !ptr.is_null() {
                let inner = &*ptr;
                match cmd_id {
                    ID_PRESET_EXPLAIN => {
                        let text = HSTRING::from("Explain the solution and concepts in the screenshot.");
                        let _ = SetWindowTextW(inner.edit_hwnd, PCWSTR(text.as_ptr()));
                        let _ = SetFocus(Some(inner.edit_hwnd));
                        let _ = SendMessageW(inner.edit_hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                    }
                    ID_PRESET_CODE => {
                        let text = HSTRING::from("Write clean, runnable code for the problem in the screenshot.");
                        let _ = SetWindowTextW(inner.edit_hwnd, PCWSTR(text.as_ptr()));
                        let _ = SetFocus(Some(inner.edit_hwnd));
                        let _ = SendMessageW(inner.edit_hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                    }
                    ID_PRESET_MCQ => {
                        let text = HSTRING::from("Identify the correct option letter and answer text.");
                        let _ = SetWindowTextW(inner.edit_hwnd, PCWSTR(text.as_ptr()));
                        let _ = SetFocus(Some(inner.edit_hwnd));
                        let _ = SendMessageW(inner.edit_hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                    }
                    ID_PRESET_STEPS => {
                        let text = HSTRING::from("Provide the final answer and step-by-step formula breakdown.");
                        let _ = SetWindowTextW(inner.edit_hwnd, PCWSTR(text.as_ptr()));
                        let _ = SetFocus(Some(inner.edit_hwnd));
                        let _ = SendMessageW(inner.edit_hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                    }
                    ID_SUBMIT => {
                        let mut buf = [0u16; 1024];
                        let len = GetWindowTextW(inner.edit_hwnd, &mut buf);
                        let typed = String::from_utf16_lossy(&buf[..len as usize]).trim().to_string();

                        let capture_new = SendMessageW(inner.chk_capture_hwnd, BM_GETCHECK, None, None).0 == 1;
                        let set_as_default = SendMessageW(inner.chk_default_hwnd, BM_GETCHECK, None, None).0 == 1;

                        let question = if typed.is_empty() {
                            "Analyze the screenshot and solve this problem.".to_string()
                        } else {
                            typed
                        };

                        let sub = CustomQuestionSubmission {
                            question,
                            capture_new,
                            set_as_default,
                        };

                        let _ = inner.tx.send(sub);
                        let _ = ShowWindow(hwnd, SW_HIDE);

                        let _ = PostThreadMessageW(
                            inner.main_thread_id,
                            crate::hotkeys::WM_APP_CUSTOM_QUESTION_SUBMIT,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                    ID_CANCEL => {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_NCHITTEST => {
            let pt_x = (lp.0 & 0xFFFF) as i16 as i32;
            let pt_y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;

            let mut win_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut win_rect);
            let local_y = pt_y - win_rect.top;
            let local_x = pt_x - win_rect.left;

            // Close button at top right
            if local_x >= win_rect.right - win_rect.left - 30 && local_y <= 30 {
                LRESULT(HTCLIENT as isize)
            } else if local_y <= 42 {
                // Drag caption area
                LRESULT(HTCAPTION as isize)
            } else {
                LRESULT(HTCLIENT as isize)
            }
        }
        WM_LBUTTONUP => {
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            // Close button clicked
            if x >= client_rect.right - 30 && y <= 30 {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DialogInner;
            let bg_brush = if !ptr.is_null() {
                (*ptr).bg_brush.0
            } else {
                CreateSolidBrush(COLORREF(0x001E1A18))
            };

            let _ = FillRect(hdc, &client_rect, bg_brush);

            // Gold/Amber top accent stripe
            let stripe_rect = RECT { left: 0, top: 0, right: client_rect.right, bottom: 2 };
            let stripe_brush = CreateSolidBrush(COLORREF(0x0000D7FF));
            let _ = FillRect(hdc, &stripe_rect, stripe_brush);
            let _ = DeleteObject(stripe_brush.into());

            // Subtle border
            let border_brush = CreateSolidBrush(COLORREF(0x00443C36));
            let _ = FrameRect(hdc, &client_rect, border_brush);
            let _ = DeleteObject(border_brush.into());

            let font = CreateFontW(
                14, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_TT_PRECIS,
                windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, font.into());
            let _ = SetBkMode(hdc, TRANSPARENT);

            // Header title
            let _ = SetTextColor(hdc, COLORREF(0x0000D7FF));
            let mut title_chars: Vec<u16> = "⚡ ASK CUSTOM QUESTION".encode_utf16().collect();
            let mut title_rect = RECT { left: 16, top: 6, right: client_rect.right - 36, bottom: 26 };
            let _ = DrawTextW(hdc, &mut title_chars, &raw mut title_rect, DT_VCENTER | DT_SINGLELINE);

            // Close button [✕]
            let mut close_chars: Vec<u16> = "✕".encode_utf16().collect();
            let mut close_rect = RECT { left: client_rect.right - 28, top: 6, right: client_rect.right - 8, bottom: 26 };
            let _ = SetTextColor(hdc, COLORREF(0x00888888));
            let _ = DrawTextW(hdc, &mut close_chars, &raw mut close_rect, DT_VCENTER | DT_SINGLELINE);

            // Subtitle
            let sub_font = CreateFontW(
                12, 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_TT_PRECIS,
                windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, sub_font.into());
            let _ = SetTextColor(hdc, COLORREF(0x00A09890));
            let mut sub_chars: Vec<u16> = "Type question & hit Enter. Results display directly in HUD.".encode_utf16().collect();
            let mut sub_rect = RECT { left: 16, top: 28, right: client_rect.right - 16, bottom: 48 };
            let _ = DrawTextW(hdc, &mut sub_chars, &raw mut sub_rect, DT_VCENTER | DT_SINGLELINE);

            // Footer keyboard hint
            let hint_font = CreateFontW(
                11, 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_TT_PRECIS,
                windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0),
                CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, hint_font.into());
            let _ = SetTextColor(hdc, COLORREF(0x00787068));
            let mut hint_chars: Vec<u16> = "⌨ Enter to solve • Esc to close".encode_utf16().collect();
            let mut hint_rect = RECT { left: 16, top: 194, right: client_rect.right - 16, bottom: 216 };
            let _ = DrawTextW(hdc, &mut hint_chars, &raw mut hint_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(font.into());
            let _ = DeleteObject(sub_font.into());
            let _ = DeleteObject(hint_font.into());

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
