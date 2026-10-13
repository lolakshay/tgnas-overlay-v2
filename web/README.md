# Screenshot Dashboard & Upload API

This is the remote upload backend and responsive web dashboard for the **Transparent Overlay Capture (Windows)** utility.

---

## 1. Environment Setup

Copy `.env.example` to `.env` inside the `web/` folder and populate:

```text
BLOB_READ_WRITE_TOKEN=your_vercel_blob_token
DEVICE_UPLOAD_TOKEN=choose_a_long_random_device_token
DASHBOARD_PASSWORD=choose_a_strong_dashboard_password
```

---

## 2. Local Development

To run the serverless backend and frontend locally:

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Start the local Vercel Dev server** (requires the `vercel` CLI installed: `npm i -g vercel`):
   ```bash
   vercel dev
   ```
   This compiles the TypeScript files on the fly and starts a local emulator at `http://localhost:3000`.

---

## 3. Vercel Deployment

Deploy the dashboard to Vercel in seconds:

1. Log in to Vercel (`vercel login`).
2. Run the deployment command inside the `web/` directory:
   ```bash
   vercel
   ```
3. Set the environment variables (`BLOB_READ_WRITE_TOKEN`, `DEVICE_UPLOAD_TOKEN`, `DASHBOARD_PASSWORD`) in your Vercel Project Settings.
4. For production release:
   ```bash
   vercel --prod
   ```

---

## 4. Rust App Integration

Once your Vercel instance is deployed, copy the production URL (e.g. `https://your-app.vercel.app`) and configure `config.toml` of the desktop Rust application:

```toml
[upload]
enabled = true
url = "https://your-app.vercel.app/api/upload"
token = "your-device-upload-token"
```

---

## 5. Mobile Testing

The dashboard uses responsive layouts, CSS Flexbox/Grids, and native dialog elements to provide a premier experience on mobile:

*   **Portrait Orientation**: Features a large, aspect-ratio-contained single column view showing the latest capture immediately at full width, followed by the historical captures grid.
*   **Landscape Orientation**: Automatically shifts columns to display the latest capture and previous captures side-by-side to make the best use of horizontal viewport space.
*   **Touch Targets**: Buttons, input boxes, and card previews have a minimum height/width of 44px to satisfy mobile tap guidelines.
*   **Automatic Polling Suspension**: The dashboard polls the server every 4 seconds to check for new captures. To preserve battery and Vercel API operation quotas, polling is automatically paused when the browser tab is minimized or hidden.

---

## 6. Security Notes

*   **HTTPS Enforced**: All requests to Vercel endpoints are secured with TLS.
*   **Stateless Cookie-based Session**: Authenticated web dashboard access is maintained via an HTTP-Only, Secure, SameSite=Strict cookie containing a signed cryptographically-validated timestamp. No session database is required.
*   **Authorized Uploads Only**: The upload API verifies that the client sends a `Bearer` token matching the server's `DEVICE_UPLOAD_TOKEN` before parsing request bodies.
*   **Strict Deletions**: Deletion parameters are checked to ensure they match paths starting with `screenshots/` and contain no traversal paths (`..`), protecting other stored blob objects.
*   **MIME-Type & Payload Checks**: The API rejects any files that are not `image/png` or exceed the configurable upload limit.
