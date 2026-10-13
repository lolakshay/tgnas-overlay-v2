@echo off
title Screenshot Dashboard Server
echo Cleaning up previous instances on port 3000...
for /f "tokens=5" %%a in ('netstat -aon ^| findstr :3000 ^| findstr LISTENING') do taskkill /f /pid %%a 2>nul

echo.
echo ===================================================
echo   SCREENSHOT DASHBOARD SERVER
echo ===================================================
echo.
echo Server URLs:
echo   - http://localhost:3000 (Local PC)
for /f "tokens=2 delims=:" %%a in ('ipconfig ^| findstr /i "IPv4"') do (
    for /f "tokens=1" %%b in ("%%a") do echo   - http://%%b:3000 (Mobile/Tablet on same Wi-Fi)
)
echo.
echo Starting server...
echo ===================================================
cd /d "%~dp0web"
node server.js
pause
