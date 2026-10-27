use anyhow::{anyhow, Result};
use std::sync::Mutex;
use windows::Win32::{
    Foundation::LRESULT,
    UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{
            CallNextHookEx, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
            HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP,
            WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

pub const WM_APP_SCREENSHOT: u32 = WM_APP + 1;
pub const WM_APP_TOGGLE_ANSWERS: u32 = WM_APP + 2;
pub const WM_APP_TOGGLE_OVERLAY: u32 = WM_APP + 3;
pub const WM_APP_EMERGENCY_HIDE: u32 = WM_APP + 4;
pub const WM_APP_LLM_UPDATE: u32 = WM_APP + 5;
pub const WM_APP_PREV_HISTORY: u32 = WM_APP + 6;
pub const WM_APP_NEXT_HISTORY: u32 = WM_APP + 7;
pub const WM_APP_TOGGLE_MENU: u32 = WM_APP + 8;
pub const WM_APP_OPACITY_CHANGE: u32 = WM_APP + 9;
pub const WM_APP_TOGGLE_DRAG_MODE: u32 = WM_APP + 10;
pub const WM_APP_SAVE_MENU_POS: u32 = WM_APP + 11;
pub const WM_APP_FONT_SIZE_CHANGE: u32 = WM_APP + 12;
pub const WM_APP_CYCLE_THEME: u32 = WM_APP + 13;
pub const WM_APP_EXPORT_SESSION: u32 = WM_APP + 14;
pub const WM_APP_QUIT: u32 = WM_APP + 15;
pub const WM_APP_TOGGLE_VPN: u32 = WM_APP + 16;
pub const WM_APP_CYCLE_PROMPT_MODE: u32 = WM_APP + 17;
pub const WM_APP_TOGGLE_MENU_CLICK_THROUGH: u32 = WM_APP + 18;
pub const WM_APP_OPEN_CUSTOM_QUESTION: u32 = WM_APP + 19;
pub const WM_APP_CUSTOM_QUESTION_SUBMIT: u32 = WM_APP + 20;

struct HookState {
    hook: Option<isize>,
    mouse_hook: Option<isize>,
    main_thread_id: u32,

    p_down: bool,
    o_down: bool,
    po_active: bool,

    i_down: bool,
    io_active: bool,

    k_down: bool,
    l_down: bool,
    kl_active: bool,

    comma_down: bool,
    period_down: bool,
    comma_period_active: bool,

    m_down: bool,
    n_down: bool,
    mn_active: bool,

    b_down: bool,
    bn_active: bool,

    q_down: bool,
    w_down: bool,
    qw_active: bool,

    z_down: bool,
    x_down: bool,
    zx_active: bool,

    bracket_open_down: bool,
    bracket_close_down: bool,
}

static STATE: Mutex<HookState> = Mutex::new(HookState {
    hook: None,
    mouse_hook: None,
    main_thread_id: 0,

    p_down: false,
    o_down: false,
    po_active: false,

    i_down: false,
    io_active: false,

    k_down: false,
    l_down: false,
    kl_active: false,

    comma_down: false,
    period_down: false,
    comma_period_active: false,

    m_down: false,
    n_down: false,
    mn_active: false,

    b_down: false,
    bn_active: false,

    q_down: false,
    w_down: false,
    qw_active: false,

    z_down: false,
    x_down: false,
    zx_active: false,

    bracket_open_down: false,
    bracket_close_down: false,
});

pub struct RegisteredHotkeys;

impl RegisteredHotkeys {
    pub unsafe fn register(main_thread_id: u32) -> Result<Self> {
        {
            let mut state = STATE.lock().map_err(|_| anyhow!("Lock poisoned"))?;
            state.main_thread_id = main_thread_id;
            state.p_down = false;
            state.o_down = false;
            state.po_active = false;
            state.i_down = false;
            state.io_active = false;
            state.k_down = false;
            state.l_down = false;
            state.kl_active = false;
            state.comma_down = false;
            state.period_down = false;
            state.comma_period_active = false;
            state.m_down = false;
            state.n_down = false;
            state.mn_active = false;
            state.b_down = false;
            state.bn_active = false;
            state.q_down = false;
            state.w_down = false;
            state.qw_active = false;
            state.z_down = false;
            state.x_down = false;
            state.zx_active = false;
            state.bracket_open_down = false;
            state.bracket_close_down = false;
        }

        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keyboard_proc), None, 0)
            .map_err(|e| anyhow!("Failed to install WH_KEYBOARD_LL hook: {}", e))?;

        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(ll_mouse_proc), None, 0)
            .map_err(|e| anyhow!("Failed to install WH_MOUSE_LL hook: {}", e))?;

        if let Ok(mut state) = STATE.lock() {
            state.hook = Some(hook.0 as isize);
            state.mouse_hook = Some(mouse_hook.0 as isize);
        }

        tracing::info!("Registered stealth hooks: Mouse Button 4/Back (Capture & Solve), Mouse Button 5/Forward (Toggle HUD), F6 / P+O (Solve), F7 / L+K (Answers)");
        Ok(Self)
    }

    pub fn set_esc_enabled(&mut self, _enabled: bool) {
        // No-op: Esc, Shift, Ctrl, and Alt are completely disabled for zero proctor detection!
    }
}

impl Drop for RegisteredHotkeys {
    fn drop(&mut self) {
        if let Ok(mut state) = STATE.lock() {
            if let Some(raw) = state.hook.take() {
                unsafe {
                    let _ = UnhookWindowsHookEx(HHOOK(raw as *mut std::ffi::c_void));
                }
            }
            if let Some(raw) = state.mouse_hook.take() {
                unsafe {
                    let _ = UnhookWindowsHookEx(HHOOK(raw as *mut std::ffi::c_void));
                }
            }
        }
    }
}

fn is_phys_down(vk: u32) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

unsafe extern "system" fn ll_keyboard_proc(
    n_code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = wparam.0 as u32;
        let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

        if is_down || is_up {
            let mut swallow = false;
            let kbd = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kbd.vkCode;

            if let Ok(mut state) = STATE.lock() {
                let main_thread_id = state.main_thread_id;

                // PRIMARY TRIGGERS: Function Keys (F6 - F10)
                // F-keys never produce text in input fields and are swallowed completely from reaching the browser DOM!
                match vk {
                    0x75 => { // F6: Capture & Solve
                        if is_down {
                            tracing::info!("Primary Shortcut triggered: F6 (Capture screenshot & LLM solve)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_SCREENSHOT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x76 => { // F7: Toggle Answers Display
                        if is_down {
                            tracing::info!("Primary Shortcut triggered: F7 (Toggle answer display)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_ANSWERS,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x77 => { // F8: Toggle Overlay
                        if is_down {
                            tracing::info!("Primary Shortcut triggered: F8 (Toggle overlay visibility)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_OVERLAY,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x78 => { // F9: Custom Question Dialog
                        if is_down {
                            tracing::info!("Primary Shortcut triggered: F9 (Ask Custom Question)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_OPEN_CUSTOM_QUESTION,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x79 => { // F10: Emergency Hide
                        if is_down {
                            tracing::info!("Primary Shortcut triggered: F10 (Emergency Hide)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_EMERGENCY_HIDE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x05 => { // VK_XBUTTON1 (Mouse Extra Button 1 via software driver)
                        if is_down {
                            tracing::info!("Mouse Extra Button 1 VK triggered: Capture screenshot & solve");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_SCREENSHOT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    0x06 => { // VK_XBUTTON2 (Mouse Extra Button 2 via software driver)
                        if is_down {
                            tracing::info!("Mouse Extra Button 2 VK triggered: Toggle Answer Display");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_ANSWERS,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                        swallow = true;
                    }
                    _ => {}
                }

                match vk {
                    0x50 => state.p_down = is_down,      // 'P'
                    0x4F => state.o_down = is_down,      // 'O'
                    0x49 => state.i_down = is_down,      // 'I'
                    0x4B => state.k_down = is_down,      // 'K'
                    0x4C => state.l_down = is_down,      // 'L'
                    0xBC => state.comma_down = is_down,  // ','
                    0xBE => state.period_down = is_down, // '.'
                    0x4D => state.m_down = is_down,      // 'M'
                    0x4E => state.n_down = is_down,      // 'N'
                    0x42 => state.b_down = is_down,      // 'B'
                    0x51 => state.q_down = is_down,      // 'Q'
                    0x57 => state.w_down = is_down,      // 'W'
                    0x5A => state.z_down = is_down,      // 'Z'
                    0x58 => state.x_down = is_down,      // 'X'
                    _ => {}
                }

                // SECONDARY TRIGGERS: Dual Key Chords (Swallowed on chord detection)
                // 1. P + O: Take screenshot & send to LLM
                if is_down && (vk == 0x50 || vk == 0x4F) {
                    let both_down = (state.p_down && state.o_down)
                        || (state.p_down && is_phys_down(0x4F))
                        || (state.o_down && is_phys_down(0x50));
                    if both_down {
                        swallow = true;
                        if !state.po_active {
                            state.po_active = true;
                            tracing::info!("Secondary Shortcut triggered: P + O (Capture screenshot & LLM solve)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_SCREENSHOT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x50 || vk == 0x4F) {
                    if state.po_active {
                        swallow = true;
                    }
                    state.po_active = false;
                }

                // 1b. I + O: Ask Custom Question (Open dialog)
                if is_down && (vk == 0x49 || vk == 0x4F) {
                    let both_down = (state.i_down && state.o_down)
                        || (state.i_down && is_phys_down(0x4F))
                        || (state.o_down && is_phys_down(0x49));
                    if both_down {
                        swallow = true;
                        if !state.io_active {
                            state.io_active = true;
                            tracing::info!("Secondary Shortcut triggered: I + O (Ask Custom Question)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_OPEN_CUSTOM_QUESTION,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x49 || vk == 0x4F) {
                    if state.io_active {
                        swallow = true;
                    }
                    state.io_active = false;
                }

                // 2. L + K: Toggle answers caption HUD
                if is_down && (vk == 0x4B || vk == 0x4C) {
                    let both_down = (state.k_down && state.l_down)
                        || (state.k_down && is_phys_down(0x4C))
                        || (state.l_down && is_phys_down(0x4B));
                    if both_down {
                        swallow = true;
                        if !state.kl_active {
                            state.kl_active = true;
                            tracing::info!("Secondary Shortcut triggered: L + K (Toggle answer display)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_ANSWERS,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x4B || vk == 0x4C) {
                    if state.kl_active {
                        swallow = true;
                    }
                    state.kl_active = false;
                }

                // 3. , + .: Toggle transparent overlay
                if is_down && (vk == 0xBC || vk == 0xBE) {
                    let both_down = (state.comma_down && state.period_down)
                        || (state.comma_down && is_phys_down(0xBE))
                        || (state.period_down && is_phys_down(0xBC));
                    if both_down {
                        swallow = true;
                        if !state.comma_period_active {
                            state.comma_period_active = true;
                            tracing::info!("Secondary Shortcut triggered: , + . (Toggle overlay visibility)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_OVERLAY,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0xBC || vk == 0xBE) {
                    if state.comma_period_active {
                        swallow = true;
                    }
                    state.comma_period_active = false;
                }

                // 4. M + N: Toggle Master Control Panel (Menu Bar)
                if is_down && (vk == 0x4D || vk == 0x4E) {
                    let both_down = (state.m_down && state.n_down)
                        || (state.m_down && is_phys_down(0x4E))
                        || (state.n_down && is_phys_down(0x4D));
                    if both_down {
                        swallow = true;
                        if !state.mn_active {
                            state.mn_active = true;
                            tracing::info!("Secondary Shortcut triggered: M + N (Toggle Menu Bar)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_MENU,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x4D || vk == 0x4E) {
                    if state.mn_active {
                        swallow = true;
                    }
                    state.mn_active = false;
                }

                // 4b. B + N: Toggle Menu Bar Click-Through (Ghost mode)
                if is_down && (vk == 0x42 || vk == 0x4E) {
                    let both_down = (state.b_down && state.n_down)
                        || (state.b_down && is_phys_down(0x4E))
                        || (state.n_down && is_phys_down(0x42));
                    if both_down {
                        swallow = true;
                        if !state.bn_active {
                            state.bn_active = true;
                            tracing::info!("Secondary Shortcut triggered: B + N (Toggle Menu Click-Through)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_TOGGLE_MENU_CLICK_THROUGH,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x42 || vk == 0x4E) {
                    if state.bn_active {
                        swallow = true;
                    }
                    state.bn_active = false;
                }

                // 5. '[' key (0xDB): Revisit previous generated answer
                if is_down && vk == 0xDB {
                    if !state.bracket_open_down {
                        state.bracket_open_down = true;
                        tracing::info!("Shortcut triggered: [ (Previous history)");
                        let _ = PostThreadMessageW(
                            main_thread_id,
                            WM_APP_PREV_HISTORY,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                    swallow = true;
                } else if is_up && vk == 0xDB {
                    state.bracket_open_down = false;
                    swallow = true;
                }

                // 6. ']' key (0xDD): Revisit next generated answer
                if is_down && vk == 0xDD {
                    if !state.bracket_close_down {
                        state.bracket_close_down = true;
                        tracing::info!("Shortcut triggered: ] (Next history)");
                        let _ = PostThreadMessageW(
                            main_thread_id,
                            WM_APP_NEXT_HISTORY,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                    swallow = true;
                } else if is_up && vk == 0xDD {
                    state.bracket_close_down = false;
                    swallow = true;
                }

                // 7. Q + W: Quit application cleanly
                if is_down && (vk == 0x51 || vk == 0x57) {
                    let both_down = (state.q_down && state.w_down)
                        || (state.q_down && is_phys_down(0x57))
                        || (state.w_down && is_phys_down(0x51));
                    if both_down {
                        swallow = true;
                        if !state.qw_active {
                            state.qw_active = true;
                            tracing::info!("Secondary Shortcut triggered: Q + W (Clean Application Exit)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_QUIT,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x51 || vk == 0x57) {
                    if state.qw_active {
                        swallow = true;
                    }
                    state.qw_active = false;
                }

                // 8. Z + X: Emergency Hide everything
                if is_down && (vk == 0x5A || vk == 0x58) {
                    let both_down = (state.z_down && state.x_down)
                        || (state.z_down && is_phys_down(0x58))
                        || (state.x_down && is_phys_down(0x5A));
                    if both_down {
                        swallow = true;
                        if !state.zx_active {
                            state.zx_active = true;
                            tracing::info!("Secondary Shortcut triggered: Z + X (Emergency Hide)");
                            let _ = PostThreadMessageW(
                                main_thread_id,
                                WM_APP_EMERGENCY_HIDE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                } else if is_up && (vk == 0x5A || vk == 0x58) {
                    if state.zx_active {
                        swallow = true;
                    }
                    state.zx_active = false;
                }
            }

            if swallow {
                return LRESULT(1);
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}

const WM_XBUTTONDOWN: u32 = 0x020B;
const WM_XBUTTONUP: u32 = 0x020C;
const WM_NCXBUTTONDOWN: u32 = 0x00AB;
const WM_NCXBUTTONUP: u32 = 0x00AC;

unsafe extern "system" fn ll_mouse_proc(
    n_code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_XBUTTONDOWN || msg == WM_XBUTTONUP || msg == WM_NCXBUTTONDOWN || msg == WM_NCXBUTTONUP {
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let xbutton = (ms.mouseData >> 16) & 0xFFFF;
            let is_down = msg == WM_XBUTTONDOWN || msg == WM_NCXBUTTONDOWN;

            let main_thread_id = if let Ok(state) = STATE.lock() {
                state.main_thread_id
            } else {
                0
            };

            if xbutton == 1 {
                // Mouse Extra Button 1 (Back / Lower thumb button): Capture screenshot & solve
                if is_down && main_thread_id != 0 {
                    tracing::info!("Mouse Extra Button 1 triggered: Capture & Solve (swallowing click)");
                    let _ = PostThreadMessageW(
                        main_thread_id,
                        WM_APP_SCREENSHOT,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
                return LRESULT(1); // SWALLOW: prevents browser from navigating back!
            } else if xbutton == 2 {
                // Mouse Extra Button 2 (Forward / Upper thumb button): Toggle Answer HUD
                if is_down && main_thread_id != 0 {
                    tracing::info!("Mouse Extra Button 2 triggered: Toggle Answer Display (swallowing click)");
                    let _ = PostThreadMessageW(
                        main_thread_id,
                        WM_APP_TOGGLE_ANSWERS,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
                return LRESULT(1); // SWALLOW: prevents browser from navigating forward!
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}

