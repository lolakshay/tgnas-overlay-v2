// ==========================================================================
// STATE MANAGEMENT & DOM SELECTION
// ==========================================================================
const state = {
  authenticated: false,
  screenshots: [],
  latestId: null,
  pollingInterval: null,
  isPolling: false,
  currentIndex: 0, // Keeps track of current slide in lightbox/earbuds
};

let silentAudio = null;

// UI Screens
const loginScreen = document.getElementById('login-screen');
const dashboardScreen = document.getElementById('dashboard-screen');

// Login Components
const loginForm = document.getElementById('login-form');
const passwordInput = document.getElementById('password-input');
const togglePasswordVisibilityBtn = document.getElementById('toggle-password-visibility');
const eyeIcon = document.getElementById('eye-icon');
const loginError = document.getElementById('login-error');
const loginSubmitBtn = document.getElementById('login-submit-btn');

// Dashboard Components
const badgeCount = document.getElementById('badge-count');
const logoutBtn = document.getElementById('logout-btn');
const latestImageContainer = document.getElementById('latest-image-container');
const latestDetails = document.getElementById('latest-details');
const latestTime = document.getElementById('latest-time');
const latestSize = document.getElementById('latest-size');
const btnViewLatest = document.getElementById('btn-view-latest');
const btnDeleteLatest = document.getElementById('btn-delete-latest');
const btnRefreshHistory = document.getElementById('btn-refresh-history');
const historyGrid = document.getElementById('history-grid');
const historyEmptyState = document.getElementById('history-empty-state');

// Lightbox Components
const lightboxViewer = document.getElementById('lightbox-viewer');
const lightboxImage = document.getElementById('lightbox-image');
const lightboxClose = document.getElementById('lightbox-close');
const lightboxCaption = document.getElementById('lightbox-caption');
const lightboxChatGPTBtn = document.getElementById('lightbox-chatgpt-btn');
const lightboxCopyBtn = document.getElementById('lightbox-copy-btn');

// ==========================================================================
// AUTHENTICATION LOGIC
// ==========================================================================

// Check Auth Status on Load
async function checkAuth() {
  try {
    const response = await fetch('/api/auth-check');
    const data = await response.json();
    
    if (data.authenticated) {
      showDashboard();
    } else {
      showLogin();
    }
  } catch (err) {
    showLogin();
  }
}

// Show/Hide Screens
function showLogin() {
  state.authenticated = false;
  stopPolling();
  dashboardScreen.classList.remove('active');
  loginScreen.classList.add('active');
  passwordInput.value = '';
  passwordInput.focus();
}

function showDashboard() {
  state.authenticated = true;
  loginScreen.classList.remove('active');
  dashboardScreen.classList.add('active');
  loadDashboardData();
  startPolling();
}

// Password Visibility toggle
togglePasswordVisibilityBtn.addEventListener('click', () => {
  if (passwordInput.type === 'password') {
    passwordInput.type = 'text';
    eyeIcon.innerHTML = `
      <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/>
      <line x1="1" y1="1" x2="23" y2="23"/>
    `;
  } else {
    passwordInput.type = 'password';
    eyeIcon.innerHTML = `
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
      <circle cx="12" cy="12" r="3"/>
    `;
  }
});

// Login Form Submit
loginForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  
  const password = passwordInput.value;
  loginError.classList.add('hidden');
  loginSubmitBtn.disabled = true;
  loginSubmitBtn.querySelector('span').textContent = 'Authenticating...';

  try {
    const response = await fetch('/api/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password }),
    });

    const data = await response.json();
    if (response.ok && data.success) {
      showDashboard();
      initSilentAudio(); // Initialise audio loops upon valid user interaction
    } else {
      loginError.textContent = data.error || 'Invalid credentials';
      loginError.classList.remove('hidden');
    }
  } catch (err) {
    loginError.textContent = 'Server connection failed';
    loginError.classList.remove('hidden');
  } finally {
    loginSubmitBtn.disabled = false;
    loginSubmitBtn.querySelector('span').textContent = 'Unlock Dashboard';
  }
});

// Logout Action
logoutBtn.addEventListener('click', async () => {
  try {
    await fetch('/api/logout', { method: 'POST' });
  } catch (e) {
    // Ignore error, force logout client-side
  }
  showLogin();
});

// ==========================================================================
// DATA FETCHING & DISPLAY LOGIC
// ==========================================================================

async function loadDashboardData(silent = false) {
  if (!state.authenticated) return;
  
  try {
    const response = await fetch('/api/screenshots');
    if (!response.ok) {
      if (response.status === 401) {
        showLogin();
        return;
      }
      throw new Error('Could not fetch screenshots');
    }
    
    const data = await response.json();
    state.screenshots = data.screenshots || [];
    renderDashboard(silent);
  } catch (err) {
    if (!silent) {
      alert('Unable to load screenshots. Try refreshing the page.');
    }
  }
}

// Formats file sizes cleanly
function formatBytes(bytes, decimals = 2) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

// Formats timestamp to user friendly format
function formatTimestamp(isoString) {
  const d = new Date(isoString);
  return d.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric'
  }) + ' @ ' + d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  });
}

// Main Render Function
function renderDashboard(silent = false) {
  const count = state.screenshots.length;
  badgeCount.textContent = `${count} Capture${count !== 1 ? 's' : ''}`;

  if (count === 0) {
    state.latestId = null;
    latestImageContainer.innerHTML = '<div class="placeholder-text">Waiting for screenshots...</div>';
    latestImageContainer.classList.add('empty');
    latestDetails.classList.add('hidden');
    historyGrid.innerHTML = '';
    historyEmptyState.classList.remove('hidden');
    return;
  }

  historyEmptyState.classList.add('hidden');
  const latest = state.screenshots[0];
  const previous = state.screenshots.slice(1);

  // 1. Render Latest Screenshot (if changed)
  if (state.latestId !== latest.id) {
    state.latestId = latest.id;
    
    latestImageContainer.classList.remove('empty');
    latestImageContainer.innerHTML = `
      <img id="latest-img" src="${latest.url}" alt="Latest Desktop Capture" loading="eager">
    `;
    
    // Wire click event for zoom
    document.getElementById('latest-img').addEventListener('click', () => {
      openLightbox(latest.url, latest.createdAt, 0);
    });

    // Wire actions
    btnViewLatest.onclick = () => openLightbox(latest.url, latest.createdAt, 0);
    btnDeleteLatest.onclick = () => deleteScreenshot(latest.id);

    // Copy Action
    const btnCopyLatest = document.getElementById('btn-copy-latest');
    if (btnCopyLatest) {
      btnCopyLatest.onclick = () => copyImageToClipboard(latest.url, btnCopyLatest);
    }

    // ChatGPT Action
    const btnChatGPTLatest = document.getElementById('btn-chatgpt-latest');
    if (btnChatGPTLatest) {
      btnChatGPTLatest.onclick = () => askChatGPT(latest.url);
    }

    // Info details
    latestTime.textContent = formatTimestamp(latest.createdAt);
    latestSize.textContent = formatBytes(latest.size);
    latestDetails.classList.remove('hidden');
  }

  // 2. Render History Grid (Newest-first, lazy loaded)
  historyGrid.innerHTML = '';
  
  if (previous.length === 0) {
    historyGrid.innerHTML = '<div class="empty-state" style="grid-column: 1/-1;"><p>No previous captures</p></div>';
    return;
  }

  previous.forEach((shot, index) => {
    const card = document.createElement('div');
    card.className = 'card';
    card.innerHTML = `
      <div class="card-preview">
        <img src="${shot.url}" alt="Capture ${shot.id.slice(0, 8)}" loading="lazy">
      </div>
      <div class="card-details">
        <span class="card-time">${new Date(shot.createdAt).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' })}</span>
        <div class="card-meta">
          <span>${formatBytes(shot.size, 1)}</span>
          <button class="card-delete-btn" aria-label="Delete screenshot">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="3 6 5 6 21 6"></polyline>
              <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"></path>
            </svg>
          </button>
        </div>
      </div>
    `;

    // Click on preview open Lightbox (index is shifted by 1 since 'previous' excludes item 0)
    card.querySelector('.card-preview').addEventListener('click', () => {
      openLightbox(shot.url, shot.createdAt, index + 1);
    });

    // Delete button click
    card.querySelector('.card-delete-btn').addEventListener('click', (e) => {
      e.stopPropagation();
      deleteScreenshot(shot.id);
    });

    historyGrid.appendChild(card);
  });
}

// Delete Screenshot Action
async function deleteScreenshot(id) {
  const confirmDelete = confirm('Are you sure you want to permanently delete this capture?');
  if (!confirmDelete) return;

  try {
    const response = await fetch(`/api/screenshots/${id}`, {
      method: 'DELETE',
    });

    if (response.ok) {
      // Optimistic locally update state
      state.screenshots = state.screenshots.filter((s) => s.id !== id);
      renderDashboard();
    } else {
      const data = await response.json();
      alert('Delete failed: ' + (data.error || 'Server error'));
    }
  } catch (err) {
    alert('Delete action failed. Check your internet connection.');
  }
}

// Manual Refresh
btnRefreshHistory.addEventListener('click', () => {
  loadDashboardData();
});

// ==========================================================================
// POLLING (AUTO REFRESH)
// ==========================================================================

function startPolling() {
  if (state.pollingInterval) clearInterval(state.pollingInterval);
  
  state.isPolling = true;
  state.pollingInterval = setInterval(async () => {
    if (document.visibilityState === 'hidden') return;
    await loadDashboardData(true);
  }, 4000); // Poll every 4 seconds
}

function stopPolling() {
  state.isPolling = false;
  if (state.pollingInterval) {
    clearInterval(state.pollingInterval);
    state.pollingInterval = null;
  }
}

// Monitor document visibility to pause/resume polling
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && state.authenticated && !state.isPolling) {
    startPolling();
    loadDashboardData(true);
  }
});

// ==========================================================================
// FULLSCREEN LIGHTBOX DIALOG HANDLERS
// ==========================================================================

function openLightbox(url, createdAt, index = 0) {
  state.currentIndex = index;
  lightboxImage.src = url;
  lightboxCaption.textContent = formatTimestamp(createdAt);
  
  // Wire Action clicks on lightbox footer
  if (lightboxCopyBtn) {
    lightboxCopyBtn.onclick = () => copyImageToClipboard(url, lightboxCopyBtn);
  }
  if (lightboxChatGPTBtn) {
    lightboxChatGPTBtn.onclick = () => askChatGPT(url);
  }

  lightboxViewer.showModal();
  
  initSilentAudio(); // Enforce silent loop is initialized on zoom gesture
  updateMediaMetadata(state.screenshots[index], index);
}

function closeLightbox() {
  lightboxViewer.close();
  lightboxImage.src = '';
  lightboxCaption.textContent = '';
}

// Event Listeners for Lightbox
lightboxClose.addEventListener('click', closeLightbox);

// Close on clicking backdrop overlay area
lightboxViewer.addEventListener('click', (e) => {
  if (e.target === lightboxViewer || e.target.classList.contains('lightbox-overlay') || e.target.classList.contains('lightbox-content')) {
    closeLightbox();
  }
});

// Keyboard navigation (Escape is handled natively by <dialog>)
lightboxViewer.addEventListener('cancel', () => {
  lightboxImage.src = '';
  lightboxCaption.textContent = '';
});

// ==========================================================================
// CHATGPT INTEG & DEEP LINKING
// ==========================================================================

function askChatGPT(imageUrl) {
  // Configurable template prompt
  const predefinedPrompt = "Analyze this image and explain what is happening, transcribing any text if necessary:";
  const fullText = `${predefinedPrompt}\n\n${imageUrl}`;
  const targetUrl = `https://chatgpt.com/?q=${encodeURIComponent(fullText)}`;
  
  window.open(targetUrl, '_blank');
}

// ==========================================================================
// CLIPBOARD IMAGE COPY LOGIC
// ==========================================================================

function fallbackCopyText(text) {
  const textArea = document.createElement("textarea");
  textArea.value = text;
  textArea.style.position = "fixed";
  textArea.style.left = "-9999px";
  document.body.appendChild(textArea);
  textArea.focus();
  textArea.select();
  try {
    const successful = document.execCommand('copy');
    return successful;
  } catch (err) {
    console.error('Fallback copy text failed: ', err);
    return false;
  } finally {
    document.body.removeChild(textArea);
  }
}

async function copyImageToClipboard(imageUrl, btn) {
  const originalHtml = btn.innerHTML;
  btn.disabled = true;
  btn.style.opacity = '0.7';
  
  // Use relative URL path for fetch to prevent CORS/origin blocking
  let targetUrl = imageUrl;
  try {
    const urlObj = new URL(imageUrl, window.location.origin);
    targetUrl = urlObj.pathname;
  } catch (e) {
    // Fallback to original imageUrl if parsing fails
  }

  try {
    // ClipboardItem and navigator.clipboard.write are restricted to HTTPS/Localhost secure contexts
    if (!navigator.clipboard || !window.ClipboardItem) {
      throw new Error("Clipboard API not available in unsecure browser context (requires HTTPS)");
    }

    const response = await fetch(targetUrl);
    if (!response.ok) throw new Error(`HTTP error fetching image: ${response.status}`);
    const blob = await response.blob();
    
    // Write image blob directly to browser clipboard
    await navigator.clipboard.write([
      new ClipboardItem({
        [blob.type]: blob
      })
    ]);
    
    // Visual success indicator
    btn.innerHTML = `
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="20 6 9 17 4 12"></polyline>
      </svg>
      <span>Copied!</span>
    `;
  } catch (err) {
    console.warn('Clipboard image copy failed, using text URL fallback...', err);
    // Fallback: copy the direct image link to clipboard
    const copiedText = fallbackCopyText(imageUrl);
    if (copiedText) {
      btn.innerHTML = `
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
          <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"></path>
        </svg>
        <span>Link Copied!</span>
      `;
    } else {
      btn.innerHTML = `<span>Copy Failed</span>`;
    }
  } finally {
    setTimeout(() => {
      btn.innerHTML = originalHtml;
      btn.disabled = false;
      btn.style.opacity = '';
    }, 2500);
  }
}

// ==========================================================================
// EARBUDS CONTROLS & MEDIA SESSION API
// ==========================================================================

// Programmatically generates a 60-second silent PCM WAV file
function generateSilenceBlobUrl(durationSeconds = 60) {
  const sampleRate = 8000;
  const numChannels = 1;
  const bitsPerSample = 8;
  const blockAlign = numChannels * (bitsPerSample / 8);
  const byteRate = sampleRate * blockAlign;
  const dataSize = durationSeconds * byteRate;
  
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);
  
  const writeString = (offset, string) => {
    for (let i = 0; i < string.length; i++) {
      view.setUint8(offset + i, string.charCodeAt(i));
    }
  };
  
  writeString(0, 'RIFF');
  view.setUint32(4, 36 + dataSize, true);
  writeString(8, 'WAVE');
  writeString(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); // PCM format
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeString(36, 'data');
  view.setUint32(40, dataSize, true);
  
  // 128 is center value (silence) for 8-bit unsigned PCM
  for (let i = 0; i < dataSize; i++) {
    view.setUint8(44 + i, 128);
  }
  
  const blob = new Blob([buffer], { type: 'audio/wav' });
  return URL.createObjectURL(blob);
}

// Starts a looping silent audio track to enable Bluetooth hardware media keys
function initSilentAudio() {
  if (silentAudio) return; // Already initialized

  // Create a 60-second real WAV loop so the browser treats it as a persistent media source
  const silenceUrl = generateSilenceBlobUrl(60);
  silentAudio = new Audio(silenceUrl);
  silentAudio.loop = true;
  
  silentAudio.play()
    .then(() => {
      setupMediaSession();
    })
    .catch((err) => {
      console.warn("Silent audio autoplay blocked. Waiting for gesture...", err);
    });
}

function setupMediaSession() {
  if (!('mediaSession' in navigator)) return;

  navigator.mediaSession.playbackState = 'playing';

  // Earbuds Pause/Play action (Intercepted to toggle Lightbox state)
  navigator.mediaSession.setActionHandler('pause', () => {
    toggleLightboxForEarbuds();
    // Keep playback active to retain control of hardware key hooks
    if (silentAudio) {
      silentAudio.play().then(() => {
        navigator.mediaSession.playbackState = 'playing';
      });
    }
  });

  navigator.mediaSession.setActionHandler('play', () => {
    toggleLightboxForEarbuds();
    if (silentAudio) {
      silentAudio.play().then(() => {
        navigator.mediaSession.playbackState = 'playing';
      });
    }
  });

  // Next Track (Earbud next click/swipe)
  navigator.mediaSession.setActionHandler('nexttrack', () => {
    navigateScreenshots(1);
  });

  // Previous Track (Earbud back/rewind click/swipe)
  navigator.mediaSession.setActionHandler('previoustrack', () => {
    navigateScreenshots(-1);
  });
}

function toggleLightboxForEarbuds() {
  if (state.screenshots.length === 0) return;
  
  if (lightboxViewer.open) {
    closeLightbox();
  } else {
    const shot = state.screenshots[state.currentIndex];
    openLightbox(shot.url, shot.createdAt, state.currentIndex);
  }
}

function navigateScreenshots(direction) {
  if (state.screenshots.length === 0) return;

  // Change index with wrap-around
  let newIndex = state.currentIndex + direction;
  if (newIndex < 0) {
    newIndex = state.screenshots.length - 1; // Wrap around to oldest
  } else if (newIndex >= state.screenshots.length) {
    newIndex = 0; // Wrap around to newest
  }

  state.currentIndex = newIndex;
  const shot = state.screenshots[state.currentIndex];

  // Refresh Lightbox state
  lightboxImage.src = shot.url;
  lightboxCaption.textContent = formatTimestamp(shot.createdAt);
  lightboxChatGPTBtn.onclick = () => askChatGPT(shot.url);

  if (!lightboxViewer.open) {
    lightboxViewer.showModal();
  }

  updateMediaMetadata(shot, state.currentIndex);
}

function updateMediaMetadata(shot, index) {
  if (!('mediaSession' in navigator) || !shot) return;

  navigator.mediaSession.metadata = new MediaMetadata({
    title: `Capture ${index + 1} of ${state.screenshots.length}`,
    artist: 'Screen Utility',
    album: formatTimestamp(shot.createdAt),
    artwork: [
      { src: shot.url, sizes: '512x512', type: 'image/png' }
    ]
  });
}

// Ensure audio hook is bound to general user clicks anywhere on page
document.addEventListener('click', () => {
  if (state.authenticated) {
    initSilentAudio();
  }
}, { once: false });

// ==========================================================================
// APPLICATION ENTRY
// ==========================================================================
document.addEventListener('DOMContentLoaded', () => {
  checkAuth();
});
