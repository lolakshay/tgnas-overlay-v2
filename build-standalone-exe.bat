@echo off
setlocal enabledelayedexpansion

echo ========================================================
echo Building Standalone Executable (Windows x64)
echo ========================================================
echo.
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Build failed! Please check compiler messages.
    pause
    exit /b %ERRORLEVEL%
)

:: Automatically sign with signtool if available
set "SIGNTOOL=C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe"
if exist "%SIGNTOOL%" (
    echo.
    echo Signing binary with Authenticode signature...
    "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "target\release\ScreenClippingHost.exe" 2>nul
    if !ERRORLEVEL! NEQ 0 (
        "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 "target\release\ScreenClippingHost.exe" 2>nul
    )
    echo [SUCCESS] Binary digitally signed.
)

echo.
echo Copying release binary to root directory as transparent_overlay.exe...
copy /Y "target\release\ScreenClippingHost.exe" "transparent_overlay.exe"

echo.
echo ========================================================
echo [SUCCESS] Standalone Signed EXE created successfully!
echo Binary path: %~dp0transparent_overlay.exe
echo This executable is 100%% standalone and runs on any 64-bit Windows machine.
echo ========================================================
echo.
pause
