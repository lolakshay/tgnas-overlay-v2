const express = require('express');
const multer = require('multer');
const cookieParser = require('cookie-parser');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
require('dotenv').config();

const app = express();
const PORT = process.env.PORT || 3000;

// Configurable environment parameters
const DEVICE_UPLOAD_TOKEN = process.env.DEVICE_UPLOAD_TOKEN || 'default_device_token';
const DASHBOARD_PASSWORD = process.env.DASHBOARD_PASSWORD || 'default_dashboard_password';
const MAX_FILE_SIZE = parseInt(process.env.MAX_FILE_SIZE || '20971520', 10); // 20 MB default

// Middleware
app.use(express.json());
app.use(cookieParser(process.env.COOKIE_SECRET || 'session_cookie_secret_key'));

// Serve frontend assets statically
app.use(express.static(path.join(__dirname, 'public')));
app.use('/uploads', express.static(path.join(__dirname, 'uploads')));

// Ensure local uploads directory exists
const uploadsDir = path.join(__dirname, 'uploads');
if (!fs.existsSync(uploadsDir)) {
  fs.mkdirSync(uploadsDir, { recursive: true });
}

// Multer setup to store incoming screenshots locally
const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    cb(null, uploadsDir);
  },
  filename: (req, file, cb) => {
    const now = new Date();
    const yyyy = now.getFullYear();
    const mm = String(now.getMonth() + 1).padStart(2, '0');
    const dd = String(now.getDate()).padStart(2, '0');
    const hhmmss =
      String(now.getHours()).padStart(2, '0') +
      String(now.getMinutes()).padStart(2, '0') +
      String(now.getSeconds()).padStart(2, '0');
    const randomSuffix = crypto.randomBytes(3).toString('hex');
    cb(null, `screenshot_${yyyy}${mm}${dd}_${hhmmss}_${randomSuffix}.png`);
  },
});

const upload = multer({
  storage,
  limits: { fileSize: MAX_FILE_SIZE },
  fileFilter: (req, file, cb) => {
    if (file.mimetype !== 'image/png') {
      return cb(new Error('Only PNG images are supported'), false);
    }
    cb(null, true);
  },
});

// Authentication Session Helpers
function createSessionToken() {
  const expiresAt = Date.now() + 7 * 24 * 60 * 60 * 1000; // 7 days
  const payload = JSON.stringify({ expiresAt });
  const hmac = crypto.createHmac('sha256', DASHBOARD_PASSWORD);
  hmac.update(payload);
  const signature = hmac.digest('hex');
  return Buffer.from(JSON.stringify({ payload, signature })).toString('base64url');
}

function isAuthenticated(req) {
  try {
    const session = req.cookies.session;
    if (!session) return false;

    const raw = Buffer.from(session, 'base64url').toString('utf8');
    const { payload, signature } = JSON.parse(raw);
    const hmac = crypto.createHmac('sha256', DASHBOARD_PASSWORD);
    hmac.update(payload);
    const expectedSignature = hmac.digest('hex');
    if (signature !== expectedSignature) return false;

    const { expiresAt } = JSON.parse(payload);
    if (Date.now() > expiresAt) return false;

    return true;
  } catch {
    return false;
  }
}

// ==========================================================================
// API ROUTING
// ==========================================================================

// 1. Auth check
app.get('/api/auth-check', (req, res) => {
  res.json({ authenticated: isAuthenticated(req) });
});

// 2. Login
app.post('/api/login', (req, res) => {
  const { password } = req.body;
  if (password !== DASHBOARD_PASSWORD) {
    return res.status(401).json({ error: 'Invalid password' });
  }

  const token = createSessionToken();
  res.cookie('session', token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    maxAge: 7 * 24 * 60 * 60 * 1000,
  });
  res.json({ success: true });
});

// 3. Logout
app.post('/api/logout', (req, res) => {
  res.clearCookie('session');
  res.json({ success: true });
});

// 4. Upload Screenshot (Rust endpoint)
app.post(
  '/api/upload',
  (req, res, next) => {
    const authHeader = req.headers.authorization;
    if (!authHeader || authHeader !== `Bearer ${DEVICE_UPLOAD_TOKEN}`) {
      return res.status(401).json({ error: 'Unauthorized' });
    }
    next();
  },
  upload.single('screenshot'),
  (req, res) => {
    if (!req.file) {
      return res.status(400).json({ error: 'No screenshot file provided' });
    }

    const fileUrl = `${req.protocol}://${req.get('host')}/uploads/${req.file.filename}`;
    res.json({
      success: true,
      id: req.file.filename,
      url: fileUrl,
      createdAt: new Date().toISOString(),
    });
  }
);

// 5. Get List of screenshots
app.get('/api/screenshots', (req, res) => {
  if (!isAuthenticated(req)) {
    return res.status(401).json({ error: 'Unauthorized' });
  }

  fs.readdir(uploadsDir, (err, files) => {
    if (err) {
      return res.status(500).json({ error: 'Failed to read uploads directory' });
    }

    const screenshots = files
      .filter((file) => file.startsWith('screenshot_') && file.endsWith('.png'))
      .map((file) => {
        const filePath = path.join(uploadsDir, file);
        const stats = fs.statSync(filePath);
        const fileUrl = `${req.protocol}://${req.get('host')}/uploads/${file}`;
        return {
          id: file,
          url: fileUrl,
          createdAt: stats.mtime.toISOString(),
          size: stats.size,
        };
      });

    // Sort newest first (mtime descending)
    screenshots.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());

    res.json({ screenshots });
  });
});

// 6. Delete a screenshot
app.delete('/api/screenshots/:id', (req, res) => {
  if (!isAuthenticated(req)) {
    return res.status(401).json({ error: 'Unauthorized' });
  }

  const { id } = req.params;
  // Prevent directory traversal
  if (id.includes('..') || path.basename(id) !== id) {
    return res.status(400).json({ error: 'Invalid file ID' });
  }

  const filePath = path.join(uploadsDir, id);
  if (!fs.existsSync(filePath)) {
    return res.status(404).json({ error: 'Screenshot not found' });
  }

  fs.unlink(filePath, (err) => {
    if (err) {
      return res.status(500).json({ error: 'Failed to delete screenshot' });
    }
    res.json({ success: true });
  });
});

// Start Server
app.listen(PORT, () => {
  console.log(`Local Server running on port ${PORT}`);
});
