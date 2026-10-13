use crate::{
    config::{CaptureConfig, DeliveryMode, TelegramConfig},
    telegram,
};
use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use std::{path::PathBuf, sync::mpsc, mem::size_of};
use windows_capture::{
    capture::{Context as CaptureContext, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, CreateCompatibleDC, DeleteDC, CreateCompatibleBitmap, SelectObject,
    BitBlt, GetDIBits, SRCCOPY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DeleteObject, HGDIOBJ
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};


/// WGC callback. The helper crate owns a Direct3D11 frame pool and capture session,
/// exposing the acquired Windows.Graphics.Capture frame as RGBA bytes.
struct OneFrame {
    sender: mpsc::SyncSender<Result<(u32, u32, Vec<u8>)>>,
}
impl GraphicsCaptureApiHandler for OneFrame {
    type Flags = mpsc::SyncSender<Result<(u32, u32, Vec<u8>)>>;
    type Error = anyhow::Error;
    fn new(ctx: CaptureContext<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { sender: ctx.flags })
    }
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let result = frame
            .buffer()
            .map_err(|e| anyhow::anyhow!(e))
            .map(|buffer| {
                let width = buffer.width();
                let height = buffer.height();
                let mut compact = Vec::new();
                let bytes = buffer.as_nopadding_buffer(&mut compact).to_vec();
                (width, height, bytes)
            });
        let _ = self.sender.send(result);
        control.stop();
        Ok(())
    }
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

static LAST_CAPTURED_JPEG: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);

pub fn spawn_capture(
    config: CaptureConfig,
    telegram_config: TelegramConfig,
    upload_config: crate::config::UploadConfig,
    llm_config: crate::config::LlmConfig,
    llm_sender: std::sync::mpsc::Sender<Result<String, String>>,
    main_thread_id: u32,
) {
    std::thread::spawn(move || {
        let result = capture_and_deliver(config, telegram_config, upload_config, &llm_config, None);
        match result {
            Ok(maybe_answer) => {
                if let Some(answer) = maybe_answer {
                    let _ = llm_sender.send(Ok(answer));
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                            main_thread_id,
                            crate::hotkeys::WM_APP_LLM_UPDATE,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                }
            }
            Err(error) => {
                tracing::error!(%error, "Screenshot / LLM failed");
                let _ = llm_sender.send(Err(error.to_string()));
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                        main_thread_id,
                        crate::hotkeys::WM_APP_LLM_UPDATE,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        }
    });
}

pub fn spawn_capture_with_custom_prompt(
    config: CaptureConfig,
    telegram_config: TelegramConfig,
    upload_config: crate::config::UploadConfig,
    llm_config: crate::config::LlmConfig,
    custom_question: String,
    capture_new: bool,
    llm_sender: std::sync::mpsc::Sender<Result<String, String>>,
    main_thread_id: u32,
) {
    std::thread::spawn(move || {
        // If user unchecked "capture_new", check if we have cached image
        if !capture_new {
            let cached_opt = LAST_CAPTURED_JPEG.lock().ok().and_then(|g| g.clone());
            if let Some(cached_bytes) = cached_opt {
                tracing::info!("Reusing existing screenshot for custom question");
                let result = crate::llm::query_openrouter(
                    &llm_config,
                    &cached_bytes,
                    "image/jpeg",
                    Some(&custom_question),
                );
                match result {
                    Ok(ans) => {
                        let _ = llm_sender.send(Ok(ans));
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_LLM_UPDATE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                    Err(e) => {
                        let _ = llm_sender.send(Err(e.to_string()));
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                                main_thread_id,
                                crate::hotkeys::WM_APP_LLM_UPDATE,
                                windows::Win32::Foundation::WPARAM(0),
                                windows::Win32::Foundation::LPARAM(0),
                            );
                        }
                    }
                }
                return;
            }
        }

        let result = capture_and_deliver(config, telegram_config, upload_config, &llm_config, Some(&custom_question));
        match result {
            Ok(maybe_answer) => {
                if let Some(answer) = maybe_answer {
                    let _ = llm_sender.send(Ok(answer));
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                            main_thread_id,
                            crate::hotkeys::WM_APP_LLM_UPDATE,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                }
            }
            Err(error) => {
                tracing::error!(%error, "Screenshot / LLM failed");
                let _ = llm_sender.send(Err(error.to_string()));
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                        main_thread_id,
                        crate::hotkeys::WM_APP_LLM_UPDATE,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        }
    });
}

fn capture_and_deliver(
    config: CaptureConfig,
    telegram_config: TelegramConfig,
    upload_config: crate::config::UploadConfig,
    llm_config: &crate::config::LlmConfig,
    custom_prompt: Option<&str>,
) -> Result<Option<String>> {
    tracing::info!("Screenshot requested");
    let monitor = Monitor::primary().context("finding primary display")?;
    let (tx, rx) = mpsc::sync_channel(1);
    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        tx,
    );
    // start_free_threaded creates the WGC session/frame pool on its own worker thread.
    let control =
        OneFrame::start_free_threaded(settings).context("starting Windows.Graphics.Capture")?;
    let (mut width, mut height, mut rgba) = rx.recv().context("waiting for first WGC frame")??;
    control.wait().context("stopping capture session")?;
    
    // Check if WGC capture returned a black screen, and execute GDI fallback!
    if is_mostly_black(&rgba) {
        tracing::info!("[INFO] Captured WGC frame is black/empty. Executing GDI fallback capture...");
        match capture_gdi() {
            Ok((gdi_w, gdi_h, gdi_rgba)) => {
                width = gdi_w;
                height = gdi_h;
                rgba = gdi_rgba;
                tracing::info!("[INFO] GDI fallback capture successful");
            }
            Err(e) => {
                tracing::error!(error = %e, "GDI fallback capture failed");
            }
        }
    }

    tracing::info!("[INFO] Screenshot captured");
    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, rgba)
        .context("invalid RGBA frame size")?;

    // 1. Prepare ultra-fast optimized JPEG for LLM vision (takes ~12ms, produces compact ~180KB payload)
    let llm_img_bytes = prepare_llm_jpeg(&image).unwrap_or_else(|_| encode_png(&image).unwrap_or_default());

    // Cache latest JPEG for follow-up custom questions
    if let Ok(mut cache) = LAST_CAPTURED_JPEG.lock() {
        *cache = Some(llm_img_bytes.clone());
    }

    // 2. Offload PNG encoding, local disk save, Telegram, and server upload to background thread
    //    This completely eliminates any waiting or retry delays before the LLM solves the screenshot!
    let image_clone = image.clone();
    let cfg_directory = config.directory.clone();
    let tele_cfg = telegram_config.clone();
    let up_cfg = upload_config.clone();
    std::thread::spawn(move || {
        if let Ok(png) = encode_png(&image_clone) {
            let save_locally = tele_cfg.mode != DeliveryMode::TelegramOnly;
            if save_locally {
                if let Ok(path) = unique_path(&cfg_directory) {
                    if let Ok(_) = std::fs::write(&path, &png) {
                        tracing::info!("[INFO] Saved locally: {}", path.display());
                    }
                }
            }
            if tele_cfg.enabled && tele_cfg.mode != DeliveryMode::LocalOnly {
                let _ = telegram::send_photo(png.clone());
            }
            if up_cfg.enabled {
                let mut delay = std::time::Duration::from_secs(2);
                for attempt in 1..=3 {
                    match upload_to_server(&up_cfg, &png) {
                        Ok(_) => {
                            tracing::info!("[INFO] Upload successful");
                            break;
                        }
                        Err(error) => {
                            tracing::warn!(%error, attempt, "Upload attempt failed");
                            if attempt < 3 {
                                std::thread::sleep(delay);
                                delay *= 2;
                            }
                        }
                    }
                }
            }
        }
    });

    // 3. Immediately query LLM solver with zero delay!
    let maybe_answer = if llm_config.enabled {
        match crate::llm::query_openrouter(llm_config, &llm_img_bytes, "image/jpeg", custom_prompt) {
            Ok(answer) => Some(answer),
            Err(e) => {
                tracing::error!(error = %e, "OpenRouter LLM query failed");
                Some(format!("LLM Error: {}", e))
            }
        }
    } else {
        None
    };
    Ok(maybe_answer)
}

fn prepare_llm_jpeg(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>> {
    let (w, h) = image.dimensions();
    let max_dim = 1600u32;
    let dyn_img = if w > max_dim || h > max_dim {
        let (new_w, new_h) = if w >= h {
            (max_dim, ((h as f32) * (max_dim as f32 / w as f32)) as u32)
        } else {
            (((w as f32) * (max_dim as f32 / h as f32)) as u32, max_dim)
        };
        let d = image::DynamicImage::ImageRgba8(image.clone());
        d.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle)
    } else {
        image::DynamicImage::ImageRgba8(image.clone())
    };

    let rgb_img = dyn_img.to_rgb8();
    let mut jpeg_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
    rgb_img.write_to(&mut cursor, image::ImageFormat::Jpeg)?;
    Ok(jpeg_bytes)
}

fn upload_to_server(upload_config: &crate::config::UploadConfig, png: &[u8]) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let form = reqwest::blocking::multipart::Form::new()
        .part(
            "screenshot",
            reqwest::blocking::multipart::Part::bytes(png.to_vec())
                .file_name("screenshot.png")
                .mime_str("image/png")?,
        );
    let response = client
        .post(&upload_config.url)
        .header("Authorization", format!("Bearer {}", upload_config.token))
        .multipart(form)
        .send()
        .context("calling upload API")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!("Upload server returned status {}: {}", status, body);
    }
    Ok(())
}

fn encode_png(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>> {
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .context("encoding PNG")?;
    Ok(output.into_inner())
}
fn unique_path(directory: &PathBuf) -> Result<PathBuf> {
    std::fs::create_dir_all(directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    for suffix in 0..10_000u32 {
        let base = chrono::Local::now()
            .format("screenshot_%Y%m%d_%H%M%S")
            .to_string();
        let name = if suffix == 0 {
            format!("{base}.png")
        } else {
            format!("{base}_{suffix}.png")
        };
        let path = directory.join(name);
        if !path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("could not allocate a unique screenshot filename")
}

fn is_mostly_black(rgba: &[u8]) -> bool {
    if rgba.is_empty() {
        return true;
    }
    let mut black_pixels = 0;
    let total_pixels = rgba.len() / 4;
    for i in 0..total_pixels {
        let r = rgba[i * 4];
        let g = rgba[i * 4 + 1];
        let b = rgba[i * 4 + 2];
        if r == 0 && g == 0 && b == 0 {
            black_pixels += 1;
        }
    }
    (black_pixels as f64 / total_pixels as f64) > 0.999
}

fn capture_gdi() -> Result<(u32, u32, Vec<u8>)> {
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.0 as usize == 0 {
            anyhow::bail!("GetDC failed");
        }
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        if hdc_mem.0 as usize == 0 {
            let _ = ReleaseDC(None, hdc_screen);
            anyhow::bail!("CreateCompatibleDC failed");
        }
        
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        
        let h_bitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        if h_bitmap.0 as usize == 0 {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            anyhow::bail!("CreateCompatibleBitmap failed");
        }
        
        let old_obj = SelectObject(hdc_mem, h_bitmap.into());
        
        if let Err(e) = BitBlt(hdc_mem, 0, 0, width, height, Some(hdc_screen), 0, 0, SRCCOPY) {
            let _ = SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(HGDIOBJ(h_bitmap.0));
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err(anyhow::anyhow!("BitBlt failed: {}", e));
        }
        
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative height for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let mut buffer = vec![0u8; (width * height * 4) as usize];
        
        let lines = GetDIBits(
            hdc_screen,
            h_bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        
        let _ = SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(HGDIOBJ(h_bitmap.0));
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        
        if lines == 0 {
            anyhow::bail!("GetDIBits failed");
        }
        
        for chunk in buffer.chunks_exact_mut(4) {
            let b = chunk[0];
            let r = chunk[2];
            chunk[0] = r;
            chunk[2] = b;
            chunk[3] = 255;
        }
        
        Ok((width as u32, height as u32, buffer))
    }
}

