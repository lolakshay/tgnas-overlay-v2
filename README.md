# Transparent Overlay Capture & LLM Vision Solver (Windows)

A native Rust utility for Windows 10/11 x64. On each launch it opens a native setup window where you can configure the status badge, fullscreen overlay, select your vision model, and provide one or more OpenRouter API keys for automatic fallback. It captures screens via Windows Graphics Capture (WGC / Direct3D with GDI fallback), queries the LLM with the screenshot, and renders the answer in a bottom-middle semi-transparent caption HUD that is completely excluded from screen captures and screen shares (`WDA_EXCLUDEFROMCAPTURE`).

---

## ⌨️ Controls & Shortcuts (100% Stealth — Zero Shift, Esc, Alt, or Ctrl)

> [!NOTE]
> All controls use dual-key simultaneous chords of regular letters and punctuation. No `Shift`, `Esc`, `Alt`, or `Ctrl` modifier keys are ever used, ensuring total invisibility to proctoring software, exam browsers, and anti-cheat listeners.

| Shortcut | Action | Description |
| --- | --- | --- |
| `P + O` | **Capture Screenshot & Solve** | Takes screenshot of primary display, encodes ultra-fast JPEG, dispatches to LLM with zero blocking delays |
| `I + O` | **Ask Custom Question** | Opens the sleek, stealth Custom Question Dialog right in the overlay to type any question & solve immediately |
| `L + K` | **Toggle Answers** | Shows or hides the floating answer caption HUD |
| `, + .` | **Toggle Overlay** | Toggles visibility of the fullscreen transparent overlay and status badge |
| `M + N` | **Toggle Master Control Panel** | Shows or hides the semi-transparent Master Control Menu Bar (also unlocks interactive mode if click-through was active) |
| `B + N` | **Toggle Menu Click-Through (Ghost)** | Toggles the Menu Bar between 100% click-through pass-through mode and interactive mode |
| `[` | **Previous Answer** | Revisit earlier generated answers/questions from history buffer |
| `]` | **Next Answer** | Move forward to newer answers in history buffer |
| `Z + X` | **Emergency Hide** | Instantly hides overlay, badge, menu bar, and caption HUD (Zero `Esc` used) |
| `Q + W` | **Quit / Exit Application** | Cleanly closes all windows and shuts down the application |
| `✕` *(Menu Bar)* | **Quit Application** | Clicking the red-accented `✕` button on the Master Menu Bar exits cleanly |

---

## ✨ Features & Design

### 1. GTA 5 Mod Menu Square Trainer Widget
- **Compact Square Mod Menu**: Replaced the long horizontal ribbon with an authentic, compact **320x352 square trainer box** inspired by classic GTA 5 mod menus (Menyoo / Stand):
  - Frosted acrylic dark carbon background with a glowing **Top Neon Gold Accent Stripe** (`#FFD700`).
  - High-contrast header banner: `⚡ CONTROL MENU` with top-right quick close button `[ ✕ ]`.
  - Inset slider readout displays for Opacity (`Opacity: 71%`), Font Size (`Font Size: 22px`), and History (`History: 1 / 5`).
  - GTA-style `< OPTION >` carousel selector tags with mode-specific neon glowing accent borders.
- **Draggable Anywhere**: Drag and position the square menu anywhere across multiple monitors. Clicking the header or any margin allows smooth moving, and the position is remembered in `config.toml`.
- **Instant Controls**:
  - `[ 📸 Solve ]` / `[ 👁 Answer ]`: One-click screenshot solving and answer visibility.
  - **Quick Prompt Mode Switcher (`[ 🎯 < MODE > ]`)**:
    - **`🎯 < MCQ >` (Gold)**: `"Option <letter>: <Answer>"` — ultra-fast, minimal text.
    - **`🎯 < CODE >` (Matrix Neon Green)**: Clean, runnable, indented code (Python / Java / C++ / SQL) with dynamic token headroom expansion.
    - **`🎯 < MATH >` (Cyber Cyan)**: Final numerical answer + concise 2-line step/formula breakdown.
    - **`🎯 < T / F >` (Neon Magenta)**: Strict `TRUE` or `FALSE` answers and exact single-word fill-in-the-blank responses.
    - **`🎯 < Custom Q&A >` (Neon Purple/Pink)**: Solves using your custom question or instruction.
    - Live-swaps the underlying LLM system prompt, instruction schema, and token limits dynamically with zero need to edit `config.toml`.
  - **In-Overlay Custom Question Dialog (`[ 💬 Ask Custom Question (I+O) ]`)**:
    - Opens a translucent in-overlay input bar right on your desktop with **zero proctor detection** (`WDA_EXCLUDEFROMCAPTURE`).
    - Type any custom question and hit **Enter** to instantly analyze the screen and receive answers in the floating HUD.
    - Quick preset chips: `[ ⚡ Explain ]`, `[ 💻 Code ]`, `[ 🔍 Solve MCQ ]`, `[ 📝 Steps ]`.
    - Options to capture fresh screenshot or reuse existing screenshot for fast follow-up questions.
    - Option to save question as the active Custom Mode prompt for subsequent `P+O` captures.
  - **Live Opacity Control**: Adjust text and overlay opacity in real-time (`[-]` and `[+]` buttons).
  - **Live Font Size Scaling**: Scale floating text size smoothly from 14px to 38px with `[-] Font [+]` buttons.
  - **Live Color Tint / Theme Presets**: Cycle between instant display styles with `[ 🎨 Theme ]`:
    - **Ghost White**: Crisp bright white with cinematic drop-shadow halos.
    - **IDE Green**: Terminal / Matrix green that blends into VS Code and terminal editors.
    - **IDE Amber**: Warm amber cyber theme matching code editor syntax highlights.
    - **Low-Contrast Gray**: Very faint text looking like background watermarks or screen glare for maximum stealth.
  - **Auto-Dim / Inactivity Fade**: Automatically dims the floating text down to 10% opacity after 8 seconds of inactivity, waking up instantly when you hover the mouse or trigger any hotkey.
  - **Interactive Drag & Resize Text Area**: Click `[ ✥ Move Text Area ]` to visually drag and resize the caption box anywhere on screen. Click `[ 🔒 Lock Area ]` to lock it back to 100% click-through.
  - **Menu Click-Through / Ghost Mode (`[ 👻 Menu Click-Through: OFF / ON ]`)**:
    - One-click toggle directly on the Menu Bar to make the **Menu Bar itself 100% click-through** (`WS_EX_TRANSPARENT`).
    - While active, all mouse clicks pass straight through the menu into your browser, IDE, or exam questions underneath.
    - Outer border glows in neon purple/magenta (`⚡ GHOST MENU`) to clearly indicate ghost pass-through mode.
    - **Never stuck**: Press `B + N` or `M + N` at any time to instantly restore interactive clickability!
  - **History Browser**: Step through previous answers via `[ ◀ ]` and `[ ▶ ]` or using `[` and `]` keyboard shortcuts.
  - **Session History Export (`[ 💾 Save Session ]`)**: Dumps all captured questions, timestamps, and LLM answers into clean, formatted `sessions/exam_session_YYYY-MM-DD.md` and `.txt` files for instant review.
  - **In-App Stealth VPN Tunnel (`[ 🛡️ VPN: ON/OFF ]`)**: Live one-click toggle on the Menu Bar. Bypasses campus DNS filtering, SNI domain blocking, and firewall API blacklists via Direct-to-Edge Anycast resolution, optional Cloudflare Worker relays, or SOCKS5/HTTP proxies with **zero virtual network adapters** (100% undetectable by exam proctors).

### 2. High-Speed Output & Latency Optimization (10x Faster)
- **Zero-Wait Asynchronous Storage**: Local screenshot saving, Telegram delivery, and server uploads run detached in the background without holding up the LLM solver for a single millisecond.
- **Ultra-Fast Compact JPEG Encoding**: Downscales high-res 4K/1440p screenshots to optimal vision dimensions and encodes in ~10ms (~180KB payload instead of 8MB uncompressed PNG), slashing network upload time by 90%.
- **Persistent HTTP Connection Pooling**: Keeps TCP/TLS keep-alive connections alive to OpenRouter, eliminating DNS and TLS handshake latency on every capture.
- **Pure Standalone Single Binary**: Compiles into a completely self-contained `.exe` (`transparent_overlay.exe`) with zero external runtime requirements. Works on any Windows 10/11 machine without Rust, Python, or Node installed!

### 3. Pure Floating Text HUD (Zero Background & True Click-Through)
- **Zero Background Box**: Rendered using Windows native color keying (`LWA_COLORKEY`), completely eliminating any rectangular background or container. Only clean floating text is displayed.
- **True 100% Click-Through**: Built with `WS_EX_TRANSPARENT` across all overlay windows (`Overlay`, `StatusBadge`, and `CaptionHUD`), guaranteeing that mouse clicks, selections, scrolling, and dragging pass straight through to whatever application is beneath.
- **Cinematic Text Shadows**: Draws subtle multi-directional drop-shadows behind the text, ensuring 100% legibility on light, dark, and complex backgrounds alike.
- **Capture-Proof**: Excluded from Windows capture via `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` (invisible to OBS, Zoom, Teams, Discord, and screenshots).

### 2. Model Choosing
- Choose from preset top vision models via the dropdown in the setup window, or type any custom OpenRouter model:
  - `google/gemini-2.5-flash` (Recommended — ultra-fast multimodal reasoning)
  - `google/gemini-2.0-flash-001`
  - `openai/gpt-4o-mini`
  - `openai/gpt-4o`
  - `anthropic/claude-3.5-sonnet`
  - `meta-llama/llama-3.2-11b-vision-instruct`

### 3. Multiple API Keys with Automatic Fallback Strategy
- Provide multiple OpenRouter API keys separated by commas in the setup dialog, `config.toml`, or `.env`.
- If an API key fails (due to insufficient credits, rate limits, or network errors), the solver **automatically falls back to the next key in the list**, ensuring zero downtime during exams or tests!

---

## 🛠️ Build and Run

1. Configure keys in `.env` or enter them in the setup dialog on startup:
   ```text
   OPENROUTER_API_KEYS=sk-or-v1-key1,sk-or-v1-key2
   # or single key:
   OPENROUTER_API_KEY=sk-or-v1-key1
   ```
2. Run the application:
   ```bash
   cargo run --release
   ```

---

## ⚙️ Configuration (`config.toml`)

```toml
[overlay]
enabled = true
topmost = true
click_through = true
status_visible = true
status_text = "Active"

[capture]
directory = "screenshots"
format = "png"

[llm]
enabled = true
api_key = "" # Primary OpenRouter key
api_keys = ["sk-or-v1-key1...", "sk-or-v1-key2..."] # Fallback keys
model = "google/gemini-2.5-flash"
system_prompt = "You are an expert exam and test assistant. Only give the answer and the option only. No explanations, no markdown intro or outro. Output strictly as: Option <letter>: <Answer>"
endpoint = "https://openrouter.ai/api/v1/chat/completions"
max_tokens = 150
caption_opacity = 145 # Semi-transparent (~57% opacity, range 1-255)
font_size = 22 # Scalable font size (14px - 38px)
theme = "white" # "white" (Ghost White), "green" (IDE Green), "amber" (IDE Amber), "gray" (Low-Contrast)
auto_dim = true # Auto-dim down to 10% opacity after 8s inactivity
vpn_enabled = false # Enable stealth DNS & firewall bypass
vpn_relay_url = "" # Optional custom Cloudflare Worker edge relay URL
proxy_url = "" # Optional SOCKS5 or HTTP proxy (e.g. "socks5://127.0.0.1:10808")
```
