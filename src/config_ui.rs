use crate::config::Config;
use anyhow::{anyhow, Result};
use std::ffi::c_void;
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

const CLASS: PCWSTR = w!("System.Dwm.HostSetup");
const ID_STATUS_ENABLED: i32 = 101;
const ID_STATUS_TEXT: i32 = 102;
const ID_START_ENABLED: i32 = 103;
const ID_SAVE: i32 = 104;
const ID_CANCEL: i32 = 105;
const ID_API_KEYS: i32 = 106;
const ID_MODEL: i32 = 107;
const CHECKED: usize = 1;

pub const PRESET_MODELS: &[&str] = &[
    // Tier 1: Next-Gen Gemini Frontier Reasoning & Vision (High-effort internal thinking)
    "google/gemini-3.8-flash",
    "google/gemini-3.7-flash",
    "google/gemini-3.5-flash",
    "google/gemini-3.1-pro-preview",
    "google/gemini-4-argon",
    "google/gemini-2.5-pro",
    "google/gemini-2.5-flash",

    // Tier 2: Anthropic Frontier Reasoning & Multimodal
    "anthropic/claude-sonnet-5.5",
    "anthropic/claude-sonnet-5",
    "anthropic/claude-3.7-sonnet",
    "anthropic/claude-3.7-sonnet:thinking",
    "anthropic/claude-3.5-sonnet",

    // Tier 3: OpenAI Reasoning & Flagship Multimodal
    "openai/o3-mini",
    "openai/o3-mini-high",
    "openai/o1",
    "openai/gpt-4o",
    "openai/gpt-4o-mini",

    // Tier 4: Open-Weights Reasoning & Vision Champions
    "deepseek/deepseek-r1",
    "qwen/qwen-2.5-vl-72b-instruct",
    "meta-llama/llama-3.2-90b-vision-instruct",
    "mistralai/pixtral-large-2411",
    "x-ai/grok-2-vision-1212",

    // Tier 5: Sub-Second Speed Vision Models
    "google/gemini-2.0-flash-001",
    "anthropic/claude-3.5-haiku",
];

struct State {
    config: *mut Config,
    saved: *mut bool,
}

/// Shows the lightweight native settings dialog before the overlay is started.
/// Returns Ok(true) if the user clicked "Save and start", or Ok(false) if cancelled/closed.
pub unsafe fn show(config: &mut Config) -> Result<bool> {
    let instance: HINSTANCE = GetModuleHandleW(None)?.into();
    let wc = WNDCLASSW {
        hInstance: instance,
        lpszClassName: CLASS,
        lpfnWndProc: Some(proc),
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        ..Default::default()
    };
    let atom = RegisterClassW(&wc);
    if atom == 0 && windows::core::Error::from_thread().code().0 != 1410 {
        return Err(anyhow!("RegisterClassW failed"));
    }
    let title = HSTRING::from("System Display Setup");
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME,
        CLASS,
        PCWSTR(title.as_ptr()),
        WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        450,
        360,
        None,
        None,
        Some(instance),
        None,
    )?;
    let mut saved = false;
    let state = Box::new(State {
        config,
        saved: &mut saved,
    });
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
    create_controls(hwnd, config)?;
    let mut msg = MSG::default();
    while IsWindow(Some(hwnd)).as_bool() && GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    Ok(saved)
}

unsafe fn create_controls(hwnd: HWND, config: &Config) -> Result<()> {
    let instance: HINSTANCE = GetModuleHandleW(None)?.into();

    // Title
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("Startup configuration"),
        WS_CHILD | WS_VISIBLE,
        18,
        14,
        300,
        22,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;

    // Status badge checkbox
    CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        w!("Show status badge"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        18,
        42,
        160,
        24,
        Some(hwnd),
        Some(menu(ID_STATUS_ENABLED)),
        Some(instance),
        None,
    )?;
    let enabled = if config.overlay.status_visible {
        CHECKED
    } else {
        0
    };
    let _ = SendMessageW(
        control(hwnd, ID_STATUS_ENABLED).ok_or_else(|| anyhow!("status control missing"))?,
        BM_SETCHECK,
        Some(WPARAM(enabled)),
        Some(LPARAM(0)),
    );

    // Badge text
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("Badge text:"),
        WS_CHILD | WS_VISIBLE,
        18,
        74,
        90,
        22,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;
    let text = HSTRING::from(&config.overlay.status_text);
    CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR(text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        110,
        72,
        300,
        24,
        Some(hwnd),
        Some(menu(ID_STATUS_TEXT)),
        Some(instance),
        None,
    )?;

    // Enable transparent fullscreen overlay
    CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        w!("Enable transparent fullscreen overlay"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        18,
        104,
        290,
        24,
        Some(hwnd),
        Some(menu(ID_START_ENABLED)),
        Some(instance),
        None,
    )?;
    let start_enabled = if config.overlay.enabled { CHECKED } else { 0 };
    let _ = SendMessageW(
        control(hwnd, ID_START_ENABLED).ok_or_else(|| anyhow!("overlay control missing"))?,
        BM_SETCHECK,
        Some(WPARAM(start_enabled)),
        Some(LPARAM(0)),
    );

    // Vision model selection
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("Vision Model:"),
        WS_CHILD | WS_VISIBLE,
        18,
        138,
        95,
        22,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;
    let model_combo = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("COMBOBOX"),
        w!(""),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWN as u32),
        110,
        136,
        300,
        240,
        Some(hwnd),
        Some(menu(ID_MODEL)),
        Some(instance),
        None,
    )?;

    // Populate model options
    for model in PRESET_MODELS {
        let m_str = HSTRING::from(*model);
        let _ = SendMessageW(
            model_combo,
            CB_ADDSTRING,
            None,
            Some(LPARAM(m_str.as_ptr() as isize)),
        );
    }
    let cur_model = HSTRING::from(&config.llm.model);
    let _ = SetWindowTextW(model_combo, PCWSTR(cur_model.as_ptr()));

    // API Key(s) (comma-separated for fallback)
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("API Key(s):"),
        WS_CHILD | WS_VISIBLE,
        18,
        172,
        90,
        22,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;
    let keys_joined = config.llm.resolved_api_keys().join(", ");
    let key_text = HSTRING::from(&keys_joined);
    CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR(key_text.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        110,
        170,
        300,
        24,
        Some(hwnd),
        Some(menu(ID_API_KEYS)),
        Some(instance),
        None,
    )?;

    // Fallback info text
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("(Enter multiple keys separated by commas for fallback)"),
        WS_CHILD | WS_VISIBLE,
        110,
        198,
        300,
        18,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;

    // Shortcuts banner
    CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        w!("Shortcuts: P+O (Solve) | L+K (Ans) | ,+. (Overlay) | Q+W (Quit) | Z+X (Hide)"),
        WS_CHILD | WS_VISIBLE,
        18,
        226,
        400,
        22,
        Some(hwnd),
        None,
        Some(instance),
        None,
    )?;

    // Save and Cancel buttons
    CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        w!("Save and start"),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        210,
        265,
        110,
        30,
        Some(hwnd),
        Some(menu(ID_SAVE)),
        Some(instance),
        None,
    )?;
    CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        w!("Cancel"),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        330,
        265,
        80,
        30,
        Some(hwnd),
        Some(menu(ID_CANCEL)),
        Some(instance),
        None,
    )?;

    Ok(())
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if (wp.0 & 0xffff) as i32 == ID_SAVE => {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
            if !state.is_null() {
                let config = &mut *(*state).config;
                config.overlay.status_visible = SendMessageW(
                    control(hwnd, ID_STATUS_ENABLED).unwrap_or_default(),
                    BM_GETCHECK,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
                .0 as u32
                    == CHECKED as u32;
                config.overlay.enabled = SendMessageW(
                    control(hwnd, ID_START_ENABLED).unwrap_or_default(),
                    BM_GETCHECK,
                    Some(WPARAM(0)),
                    Some(LPARAM(0)),
                )
                .0 as u32
                    == CHECKED as u32;

                // Status text
                let mut buffer = [0u16; 256];
                let length = GetWindowTextW(
                    control(hwnd, ID_STATUS_TEXT).unwrap_or_default(),
                    &mut buffer,
                );
                config.overlay.status_text = String::from_utf16_lossy(&buffer[..length as usize])
                    .trim()
                    .to_owned();
                if config.overlay.status_text.is_empty() {
                    config.overlay.status_text = "Active".to_owned();
                }

                // Vision model
                let mut model_buf = [0u16; 256];
                let model_len = GetWindowTextW(
                    control(hwnd, ID_MODEL).unwrap_or_default(),
                    &mut model_buf,
                );
                let typed_model = String::from_utf16_lossy(&model_buf[..model_len as usize])
                    .trim()
                    .to_owned();
                if !typed_model.is_empty() {
                    config.llm.model = typed_model;
                }

                // API Key(s)
                let mut key_buf = [0u16; 1024];
                let key_len = GetWindowTextW(
                    control(hwnd, ID_API_KEYS).unwrap_or_default(),
                    &mut key_buf,
                );
                let typed_keys = String::from_utf16_lossy(&key_buf[..key_len as usize])
                    .trim()
                    .to_owned();
                if !typed_keys.is_empty() {
                    let keys: Vec<String> = typed_keys
                        .split([',', ';', '\n'])
                        .filter_map(crate::config::sanitize_key)
                        .collect();
                    config.llm.api_keys = keys.clone();
                    config.llm.api_key = keys.first().cloned().unwrap_or_default();
                }

                if let Err(error) = config.save() {
                    tracing::error!(%error, "Could not save configuration");
                }
                *(*state).saved = true;
            }
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_COMMAND if (wp.0 & 0xffff) as i32 == ID_CANCEL => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let pointer = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut State;
            if !pointer.is_null() {
                drop(Box::from_raw(pointer));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn control(parent: HWND, id: i32) -> Option<HWND> {
    GetDlgItem(Some(parent), id).ok()
}
const fn menu(id: i32) -> HMENU {
    HMENU(id as usize as *mut c_void)
}
