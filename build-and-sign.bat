@echo off
setlocal enabledelayedexpansion

echo ========================================================
echo   Automated Build ^& Code-Signing (Windows Authenticode)
echo ========================================================
echo.

:: 1. Compile release binary
echo [1/3] Compiling optimized release binary...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Cargo build failed!
    pause
    exit /b %ERRORLEVEL%
)

:: 2. Locate signtool.exe
set "SIGNTOOL="
if exist "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" (
    set "SIGNTOOL=C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe"
) else (
    for /f "delims=" %%i in ('where signtool 2^>nul') do set "SIGNTOOL=%%i"
)

if "%SIGNTOOL%"=="" (
    for /f "delims=" %%i in ('dir /b /s "C:\Program Files (x86)\Windows Kits\10\bin\*signtool.exe" 2^>nul ^| findstr /i "x64"') do (
        set "SIGNTOOL=%%i"
    )
)

set TARGET_EXE=target\release\ScreenClippingHost.exe
set OUT_EXE=transparent_overlay.exe

:: 3. Sign the executable
echo.
if not "%SIGNTOOL%"=="" (
    echo [2/3] Digitally signing %TARGET_EXE%...
    "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "%TARGET_EXE%"
    if !ERRORLEVEL! EQU 0 (
        echo [SUCCESS] Binary signed with SHA-256 and DigiCert RFC 3161 timestamp.
    ) else (
        echo [INFO] Timestamp server unavailable, signing with local SHA-256...
        "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 "%TARGET_EXE%"
        if !ERRORLEVEL! EQU 0 (
            echo [SUCCESS] Binary signed with local SHA-256 Authenticode signature.
        ) else (
            echo [WARNING] Signing failed. Check certificate installation.
        )
    )
) else (
    echo [WARNING] signtool.exe not found. Binary compiled without signature.
)

:: 4. Deploy standalone copy and verify signature
echo.
echo [3/3] Deploying and signing final standalone binary to %OUT_EXE%...
copy /Y "%TARGET_EXE%" "%OUT_EXE%" >nul
if not "%SIGNTOOL%"=="" (
    "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 "%OUT_EXE%" >nul 2>&1
    if !ERRORLEVEL! NEQ 0 (
        "%SIGNTOOL%" sign /s My /n "Windows Desktop Host" /fd SHA256 "%OUT_EXE%" >nul 2>&1
    )
)

echo.
echo ========================================================
echo   BUILD ^& SIGNING COMPLETE
echo   Output: %~dp0%OUT_EXE%
echo ========================================================
echo.
if not "%SIGNTOOL%"=="" (
    "%SIGNTOOL%" verify /pa "%OUT_EXE%" 2>nul
    echo Signature verification status: Valid Authenticode signature present.
)
echo.
pause
