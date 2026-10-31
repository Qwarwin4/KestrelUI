@echo off
setlocal
cd /d "%~dp0"

rem build.bat          debug installer in target\debug\bundle\nsis
rem build.bat dev      run from sources with hot reload
rem build.bat release  release installer in target\release\bundle\nsis

set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\.deno\bin;%LOCALAPPDATA%\Microsoft\WinGet\Links;%ProgramFiles%\nodejs;%PATH%"

set "MODE=%~1"
if "%MODE%"=="" set "MODE=debug"

call node scripts\doctor.mjs || goto :fail

if not exist node_modules (
  echo Installing dependencies...
  call npm install || goto :fail
)

if /i "%MODE%"=="dev" (
  call npm run dev || goto :fail
  exit /b 0
)

if /i "%MODE%"=="debug" (
  cargo build -p sluhk || goto :fail
  call npx tauri build --debug -c src\tauri.local.conf.json || goto :fail
  echo.
  echo Installer: target\debug\bundle\nsis
  exit /b 0
)

if /i "%MODE%"=="release" (
  cargo build --release -p sluhk || goto :fail
  if defined TAURI_SIGNING_PRIVATE_KEY (
    call npx tauri build || goto :fail
  ) else (
    echo No TAURI_SIGNING_PRIVATE_KEY: building an unsigned installer without auto-updates
    call npx tauri build -c src\tauri.local.conf.json || goto :fail
  )
  echo.
  echo Installer: target\release\bundle\nsis
  exit /b 0
)

echo Unknown mode "%MODE%". Use: build.bat [debug^|dev^|release]
exit /b 1

:fail
echo.
echo Build failed. Run setup.bat if tools are missing, then try again in a new terminal.
exit /b 1
