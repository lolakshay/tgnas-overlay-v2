#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod caption_hud;
mod capture;
mod config;
mod config_ui;
mod custom_question;
mod hotkeys;
mod llm;
mod menu_bar;
mod overlay;
mod status_badge;
mod telegram;

use anyhow::Result;
use windows::Win32::{
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG},
};

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let mut config = config::Config::load()?;
    let silent = std::env::args().any(|a| a == "--silent" || a == "-s" || a == "--headless")
        || config.overlay.start_silent;

    if !silent {
        let started = unsafe { config_ui::show(&mut config)? };
        if !started {
            tracing::info!("Startup cancelled by user");
            return Ok(());
        }
    }
    if config.capture.format.to_lowercase() != "png" {
        anyhow::bail!("only PNG is currently supported");
    }
    tracing::info!(silent, "Starting transparent overlay, master control panel, and caption HUD");
    unsafe {
        let mut overlay = overlay::Overlay::create(config.overlay.enabled && !silent, config.overlay.topmost)?;
        let mut status_badge = status_badge::StatusBadge::create(
            &config.overlay.status_text,
            config.overlay.status_visible && !silent,
            config.overlay.enabled,
        )?;

        let custom_rect = match (config.llm.hud_x, config.llm.hud_y, config.llm.hud_w, config.llm.hud_h) {
            (Some(x), Some(y), Some(w), Some(h)) => Some((x, y, w, h)),
            _ => None,
        };
        let theme = caption_hud::HudTheme::from_str(&config.llm.theme);
        let mut caption_hud = caption_hud::CaptionHud::create(
            false,
            config.llm.caption_opacity,
            custom_rect,
            config.llm.font_size as i32,
            theme,
            config.llm.auto_dim,
        )?;

        let main_thread_id = GetCurrentThreadId();
        let mut hotkeys = hotkeys::RegisteredHotkeys::register(main_thread_id)?;

        let menu_pos = match (config.overlay.menu_x, config.overlay.menu_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };
        let mut menu_bar = menu_bar::MenuBar::create(
            !silent, // In silent mode, menu starts hidden to prevent any visible windows
            config.llm.caption_opacity,
            config.llm.font_size as i32,
            theme.short_name(),
            config.llm.vpn_enabled,
            config.llm.prompt_mode,
            main_thread_id,
            menu_pos,
        )?;

        let (custom_q_tx, custom_q_rx) = std::sync::mpsc::channel::<custom_question::CustomQuestionSubmission>();
        let custom_question_dialog = custom_question::CustomQuestionDialog::create(
            main_thread_id,
            custom_q_tx,
            menu_pos,
            config.llm.caption_opacity,
        )?;

        hotkeys.set_esc_enabled(overlay.is_visible() || caption_hud.is_visible() || menu_bar.is_visible() || custom_question_dialog.is_visible());

        let (llm_tx, llm_rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let mut history: Vec<(String, String)> = Vec::new(); // (timestamp, answer)
        let mut history_index: usize = 0;

        tracing::info!("Global hooks active: P+O=Solve, L+K=Answers, ,+.=Overlay, [ / ]=History, M+N / Ctrl+M=Menu");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            match msg.message {
                hotkeys::WM_APP_SCREENSHOT => {
                    if capture::spawn_capture(
                        config.capture.clone(),
                        config.telegram.clone(),
                        config.upload.clone(),
                        config.llm.clone(),
                        llm_tx.clone(),
                        main_thread_id,
                    ) {
                        caption_hud.set_text("Analyzing screenshot with LLM...");
                        caption_hud.set_visible(true);
                        hotkeys.set_esc_enabled(true);
                    }
                }
                hotkeys::WM_APP_TOGGLE_ANSWERS => {
                    let now_visible = caption_hud.toggle_visible();
                    tracing::info!(now_visible, "Answers caption visibility toggled");
                    hotkeys.set_esc_enabled(overlay.is_visible() || now_visible || menu_bar.is_visible());
                }
                hotkeys::WM_APP_TOGGLE_OVERLAY => {
                    overlay.toggle()?;
                    let visible = overlay.is_visible();
                    if let Some(ref mut badge) = status_badge {
                        badge.set_visible(visible);
                    }
                    if !visible {
                        caption_hud.set_visible(false);
                        menu_bar.set_visible(false);
                        custom_question_dialog.hide();
                    } else {
                        menu_bar.set_visible(true);
                    }
                    hotkeys.set_esc_enabled(visible || caption_hud.is_visible() || menu_bar.is_visible());
                }
                hotkeys::WM_APP_TOGGLE_MENU => {
                    if menu_bar.is_click_through() {
                        menu_bar.set_click_through(false);
                        let banner = "🔓 Menu Bar: Interactive Mode (Clicks enabled)";
                        tracing::info!("{}", banner);
                        caption_hud.set_text(banner);
                        caption_hud.set_visible(true);
                        hotkeys.set_esc_enabled(true);
                    } else {
                        let now_visible = menu_bar.toggle_visible();
                        tracing::info!(now_visible, "Master Control Panel toggled");
                        hotkeys.set_esc_enabled(overlay.is_visible() || caption_hud.is_visible() || now_visible);
                    }
                }
                hotkeys::WM_APP_PREV_HISTORY => {
                    if !history.is_empty() && history_index > 0 {
                        history_index -= 1;
                        caption_hud.set_text(&history[history_index].1);
                        caption_hud.set_visible(true);
                        menu_bar.update_history(history_index + 1, history.len());
                        hotkeys.set_esc_enabled(true);
                    }
                }
                hotkeys::WM_APP_NEXT_HISTORY => {
                    if !history.is_empty() && history_index + 1 < history.len() {
                        history_index += 1;
                        caption_hud.set_text(&history[history_index].1);
                        caption_hud.set_visible(true);
                        menu_bar.update_history(history_index + 1, history.len());
                        hotkeys.set_esc_enabled(true);
                    }
                }
                hotkeys::WM_APP_OPACITY_CHANGE => {
                    let increase = msg.wParam.0 != 0;
                    let step = 25i32; // ~10% step
                    let current = config.llm.caption_opacity as i32;
                    let new_op = if increase {
                        (current + step).min(255) as u8
                    } else {
                        (current - step).max(30) as u8
                    };
                    config.llm.caption_opacity = new_op;
                    caption_hud.set_opacity(new_op);
                    menu_bar.update_opacity(new_op);
                    custom_question_dialog.update_opacity(new_op);
                    let _ = config.save();
                    tracing::info!(opacity = new_op, "Adjusted caption, menu bar, and custom question box opacity");
                }
                hotkeys::WM_APP_FONT_SIZE_CHANGE => {
                    let increase = msg.wParam.0 != 0;
                    let current = config.llm.font_size as i32;
                    let new_size = if increase {
                        (current + 2).min(38)
                    } else {
                        (current - 2).max(14)
                    };
                    config.llm.font_size = new_size as u32;
                    caption_hud.set_font_size(new_size);
                    menu_bar.update_font_size(new_size);
                    let _ = config.save();
                    tracing::info!(font_size = new_size, "Adjusted font size");
                }
                hotkeys::WM_APP_CYCLE_THEME => {
                    let new_theme = caption_hud.cycle_theme();
                    config.llm.theme = new_theme.to_str().to_string();
                    menu_bar.update_theme(new_theme.short_name());
                    let _ = config.save();
                    tracing::info!(theme = %new_theme.display_name(), "HUD theme changed");
                }
                hotkeys::WM_APP_TOGGLE_DRAG_MODE => {
                    let currently_dragging = caption_hud.is_drag_mode();
                    let new_drag = !currently_dragging;
                    let rect = caption_hud.set_drag_mode(new_drag);
                    menu_bar.set_drag_mode(new_drag);
                    if !new_drag {
                        config.llm.hud_x = Some(rect.0);
                        config.llm.hud_y = Some(rect.1);
                        config.llm.hud_w = Some(rect.2);
                        config.llm.hud_h = Some(rect.3);
                        let _ = config.save();
                        tracing::info!(x = rect.0, y = rect.1, w = rect.2, h = rect.3, "Caption text area repositioned and saved");
                    }
                }
                hotkeys::WM_APP_SAVE_MENU_POS => {
                    let (x, y) = menu_bar.get_pos();
                    config.overlay.menu_x = Some(x);
                    config.overlay.menu_y = Some(y);
                    let _ = config.save();
                    tracing::info!(x, y, "Saved menu bar position");
                }
                hotkeys::WM_APP_EXPORT_SESSION => {
                    if history.is_empty() {
                        caption_hud.set_text("⚠️ No answers in session to export yet (Press P + O to solve)");
                        caption_hud.set_visible(true);
                        hotkeys.set_esc_enabled(true);
                    } else {
                        let dir = std::path::Path::new("sessions");
                        let _ = std::fs::create_dir_all(dir);
                        let now = chrono::Local::now();
                        let base_name = now.format("exam_session_%Y-%m-%d").to_string();
                        let md_path = dir.join(format!("{}.md", base_name));
                        let txt_path = dir.join(format!("{}.txt", base_name));

                        // Generate clean Markdown
                        let mut md = String::new();
                        md.push_str("# 📝 Exam & Study Session Export\n\n");
                        md.push_str(&format!("- **Session Date:** {}\n", now.format("%Y-%m-%d %H:%M:%S")));
                        md.push_str(&format!("- **Vision Model:** {}\n", config.llm.model));
                        md.push_str(&format!("- **Total Answers Captured:** {}\n\n", history.len()));
                        md.push_str("---\n\n");

                        // Generate clean Plain Text
                        let mut txt = String::new();
                        txt.push_str("================================================================================\n");
                        txt.push_str("📝 EXAM & STUDY SESSION EXPORT\n");
                        txt.push_str(&format!("Session Date: {}\n", now.format("%Y-%m-%d %H:%M:%S")));
                        txt.push_str(&format!("Vision Model: {}\n", config.llm.model));
                        txt.push_str(&format!("Total Answers Captured: {}\n", history.len()));
                        txt.push_str("================================================================================\n\n");

                        for (idx, (time, ans)) in history.iter().enumerate() {
                            md.push_str(&format!("### Question / Answer #{}\n", idx + 1));
                            md.push_str(&format!("*Captured at: {}*\n\n", time));
                            md.push_str(&format!("```text\n{}\n```\n\n", ans.trim()));
                            md.push_str("---\n\n");

                            txt.push_str(&format!("--------------------------------------------------------------------------------\n"));
                            txt.push_str(&format!("[#{}] Captured at: {}\n", idx + 1, time));
                            txt.push_str(&format!("--------------------------------------------------------------------------------\n"));
                            txt.push_str(&format!("{}\n\n", ans.trim()));
                        }

                        let res_md = std::fs::write(&md_path, md);
                        let res_txt = std::fs::write(&txt_path, txt);

                        match (res_md, res_txt) {
                            (Ok(_), Ok(_)) => {
                                let confirm = format!("💾 Saved to sessions/{}.md & .txt", base_name);
                                tracing::info!("{}", confirm);
                                caption_hud.set_text(&confirm);
                                caption_hud.set_visible(true);
                                hotkeys.set_esc_enabled(true);
                            }
                            (Err(e), _) | (_, Err(e)) => {
                                let err = format!("Failed to export session: {}", e);
                                tracing::error!("{}", err);
                                caption_hud.set_text(&err);
                                caption_hud.set_visible(true);
                                hotkeys.set_esc_enabled(true);
                            }
                        }
                    }
                }
                hotkeys::WM_APP_EMERGENCY_HIDE => {
                    overlay.set_visible(false)?;
                    if let Some(ref mut badge) = status_badge {
                        badge.set_visible(false);
                    }
                    caption_hud.set_visible(false);
                    menu_bar.set_visible(false);
                    custom_question_dialog.hide();
                    hotkeys.set_esc_enabled(false);
                }
                hotkeys::WM_APP_QUIT => {
                    tracing::info!("Application exit requested by user");
                    break;
                }
                hotkeys::WM_APP_TOGGLE_VPN => {
                    config.llm.vpn_enabled = !config.llm.vpn_enabled;
                    let enabled = config.llm.vpn_enabled;
                    menu_bar.update_vpn(enabled);
                    let _ = config.save();
                    let banner = if enabled {
                        "🛡️ Stealth VPN Tunnel ON (DNS & Firewall Bypass Active)"
                    } else {
                        "🛡️ Stealth VPN Tunnel OFF (Direct Mode)"
                    };
                    tracing::info!(vpn_enabled = enabled, "{}", banner);
                    caption_hud.set_text(banner);
                    caption_hud.set_visible(true);
                    hotkeys.set_esc_enabled(true);
                }
                hotkeys::WM_APP_CYCLE_PROMPT_MODE => {
                    config.llm.prompt_mode = config.llm.prompt_mode.next();
                    let new_mode = config.llm.prompt_mode;
                    menu_bar.update_prompt_mode(new_mode);
                    let _ = config.save();
                    let banner = format!("🎯 Mode: {} — {}", new_mode.display_name(), new_mode.description());
                    tracing::info!(prompt_mode = %new_mode.display_name(), "Prompt mode cycled");
                    caption_hud.set_text(&banner);
                    caption_hud.set_visible(true);
                    hotkeys.set_esc_enabled(true);
                }
                hotkeys::WM_APP_TOGGLE_MENU_CLICK_THROUGH => {
                    let now_ct = menu_bar.toggle_click_through();
                    let banner = if now_ct {
                        "👻 Menu Bar: Click-Through ON (Pass-through active — Press M+N or B+N to unlock)"
                    } else {
                        "🔓 Menu Bar: Interactive Mode (Clicks enabled)"
                    };
                    tracing::info!(click_through = now_ct, "{}", banner);
                    caption_hud.set_text(banner);
                    caption_hud.set_visible(true);
                    hotkeys.set_esc_enabled(true);
                }
                hotkeys::WM_APP_OPEN_CUSTOM_QUESTION => {
                    let anchor = if menu_bar.is_visible() {
                        Some(menu_bar.get_pos())
                    } else {
                        None
                    };
                    custom_question_dialog.show(&config.llm.custom_prompt, anchor);
                    hotkeys.set_esc_enabled(true);
                }
                hotkeys::WM_APP_CUSTOM_QUESTION_SUBMIT => {
                    while let Ok(sub) = custom_q_rx.try_recv() {
                        let q_disp = if sub.question.len() > 40 {
                            format!("{}...", &sub.question[..37])
                        } else {
                            sub.question.clone()
                        };
                        caption_hud.set_text(&format!("Analyzing screenshot for: \"{}\"...", q_disp));
                        caption_hud.set_visible(true);
                        hotkeys.set_esc_enabled(true);

                        if sub.set_as_default {
                            config.llm.prompt_mode = config::PromptMode::Custom;
                            config.llm.custom_prompt = sub.question.clone();
                            menu_bar.update_prompt_mode(config::PromptMode::Custom);
                            let _ = config.save();
                        }

                        capture::spawn_capture_with_custom_prompt(
                            config.capture.clone(),
                            config.telegram.clone(),
                            config.upload.clone(),
                            config.llm.clone(),
                            sub.question,
                            sub.capture_new,
                            llm_tx.clone(),
                            main_thread_id,
                        );
                    }
                }
                hotkeys::WM_APP_LLM_UPDATE => {
                    while let Ok(result) = llm_rx.try_recv() {
                        match result {
                            Ok(answer) => {
                                let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
                                history.push((time_str, answer.clone()));
                                history_index = history.len() - 1;
                                caption_hud.set_text(&answer);
                                caption_hud.set_visible(true);
                                menu_bar.update_history(history.len(), history.len());
                                hotkeys.set_esc_enabled(true);
                            }
                            Err(err) => {
                                caption_hud.set_text(&format!("Error: {}", err));
                                caption_hud.set_visible(true);
                                hotkeys.set_esc_enabled(true);
                            }
                        }
                    }
                }
                _ => {}
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
    };

    #[test]
    fn test_hotkey_registration() {
        unsafe {
            // Test registration of virtual key 'Q' (0x51) without modifiers
            let res = RegisterHotKey(None, 9999, HOT_KEY_MODIFIERS(0), 0x51);
            assert!(res.is_ok(), "Failed to register test hotkey: {:?}", res);
            let unreg = UnregisterHotKey(None, 9999);
            assert!(unreg.is_ok(), "Failed to unregister test hotkey: {:?}", unreg);
        }
    }
}
