@echo off
setlocal EnableExtensions
cd /d "%~dp0"

rem build.bat           menu: press 1-4 to pick what to do
rem build.bat debug     install what is missing, build a debug installer
rem build.bat release   same, release installer (slower, smaller and faster app)
rem build.bat dev       install what is missing, run from sources with hot reload
rem build.bat deps      only install / check the build tools
rem Everything is logged to build.log next to this file.

set "MODE=%~1"
if "%MODE%"=="" call :menu
if "%MODE%"=="" exit /b 0
set "ROOT=%~dp0"
set "LOG=%~dp0build.log"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSSETUP=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\setup.exe"
set "KITS=%ProgramFiles(x86)%\Windows Kits\10\Lib"

echo KestrelUI build, mode: %MODE%> "%LOG%"
echo.
echo KestrelUI build (%MODE%). Full log: build.log
echo.

if /i not "%MODE%"=="debug" if /i not "%MODE%"=="release" if /i not "%MODE%"=="dev" if /i not "%MODE%"=="deps" (
  echo Unknown mode "%MODE%". Use: build.bat [debug^|release^|dev^|deps]
  goto :end_fail
)

call :deps || goto :end_fail
if /i "%MODE%"=="deps" goto :end_ok

echo.
echo [build] npm install (also generates the core library, first time takes a while)
call :npm_install || goto :fail

if /i "%MODE%"=="dev" (
  echo [build] starting dev mode, close the app or press Ctrl+C to stop
  call npm run dev
  goto :end_ok
)

if /i "%MODE%"=="release" (
  set "PROFILE=release"
  set "CARGO_FLAGS=--release"
  set "TAURI_FLAGS="
) else (
  set "PROFILE=debug"
  set "CARGO_FLAGS="
  set "TAURI_FLAGS=--debug"
)

echo [build] hook dll
call :live cargo build %CARGO_FLAGS% -p sluhk || goto :fail

echo [build] app and installer
echo         Rust compiles ~700 crates the first time, 10-30 minutes. Lines keep scrolling,
echo         that is progress. Do not press Ctrl+C or close the window.
if defined TAURI_SIGNING_PRIVATE_KEY (
  call :live npx tauri build %TAURI_FLAGS% || goto :fail
) else (
  call :live npx tauri build %TAURI_FLAGS% -c src\tauri.local.conf.json || goto :fail
)

echo.
echo Done.
echo   Installer: target\%PROFILE%\bundle\nsis\
echo   App:       target\%PROFILE%\kestrelui.exe
start "" "%~dp0target\%PROFILE%\bundle\nsis"
goto :end_ok


:deps
echo [deps] checking build tools
call :refresh_path

where winget >nul 2>&1
if errorlevel 1 (
  echo   winget is missing. Install "App Installer" from Microsoft Store, then run this again.
  exit /b 1
)

call :has_msvc
if errorlevel 1 (
  echo   MSVC + Windows SDK: installing Visual Studio Build Tools, accept the admin prompt
  call :winget Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  call :has_msvc || call :add_vctools
  call :has_msvc || (
    echo   MSVC or the Windows SDK is still missing. Open "Visual Studio Installer",
    echo   modify Build Tools 2022 and tick "Desktop development with C++".
    exit /b 1
  )
)
echo   ok  MSVC + Windows SDK

call :ensure rustup Rustlang.Rustup || exit /b 1
call :ensure node OpenJS.NodeJS.LTS || exit /b 1
call :ensure deno DenoLand.Deno || exit /b 1
call :ensure git Git.Git || exit /b 1

reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" >nul 2>&1
if errorlevel 1 (
  reg query "HKCU\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" >nul 2>&1
  if errorlevel 1 call :winget Microsoft.EdgeWebView2Runtime
)
echo   ok  WebView2

set "NODE_MAJOR=0"
for /f %%v in ('node -p "parseInt(process.versions.node)"') do set "NODE_MAJOR=%%v"
if %NODE_MAJOR% LSS 22 (
  echo   Node.js %NODE_MAJOR% is too old, need 22 or newer
  call :winget OpenJS.NodeJS.LTS
  call :refresh_path
)

echo   Rust toolchain from rust-toolchain.toml (downloads on first run)
call :run rustup show active-toolchain || (
  call :run rustup toolchain install || exit /b 1
)
call :run rustup component add rustfmt
exit /b 0


rem --- helpers ---

:menu
echo.
echo   KestrelUI
echo.
echo   1  Debug build      installer for testing, builds faster
echo   2  Release build    final installer, smaller and faster app
echo   3  Dev mode         run from sources with hot reload
echo   4  Install tools    only install and check the build tools
echo   0  Exit
echo.
choice /c 12340 /n /m "Choose 1-4 or 0: "
if errorlevel 5 exit /b 0
if errorlevel 4 set "MODE=deps"& exit /b 0
if errorlevel 3 set "MODE=dev"& exit /b 0
if errorlevel 2 set "MODE=release"& exit /b 0
set "MODE=debug"
exit /b 0

:ensure
rem %1 command to look for, %2 winget package id
where %1 >nul 2>&1
if not errorlevel 1 (
  echo   ok  %1
  exit /b 0
)
echo   %1: installing %2
call :winget %2
call :refresh_path
where %1 >nul 2>&1
if errorlevel 1 (
  echo   %1 is still not found after installing %2. See build.log.
  exit /b 1
)
echo   ok  %1
exit /b 0

:winget
rem winget exits non-zero for "already installed", so presence is checked afterwards instead
echo winget install %*>> "%LOG%"
winget install -e --silent --accept-package-agreements --accept-source-agreements --id %* >> "%LOG%" 2>&1
exit /b 0

:has_msvc
if not exist "%VSWHERE%" exit /b 1
set "VSPATH="
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSPATH=%%i"
if not defined VSPATH exit /b 1
dir /s /b "%KITS%\kernel32.lib" >nul 2>&1 || exit /b 1
exit /b 0

:add_vctools
if not exist "%VSWHERE%" exit /b 1
set "VSPATH="
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -property installationPath`) do set "VSPATH=%%i"
if not defined VSPATH exit /b 1
echo   adding the C++ workload to the existing Build Tools
"%VSSETUP%" modify --installPath "%VSPATH%" --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart --wait >> "%LOG%" 2>&1
exit /b 0

:refresh_path
rem pick up tools installed a moment ago without opening a new terminal
for /f "usebackq delims=" %%p in (`powershell -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')"`) do set "PATH=%%p"
set "PATH=%USERPROFILE%\.cargo\bin;%USERPROFILE%\.deno\bin;%LOCALAPPDATA%\Microsoft\WinGet\Links;%ProgramFiles%\nodejs;%PATH%"
exit /b 0

:npm_install
call :live npm install && exit /b 0
rem EBUSY/EPERM: something still holds node_modules, usually node or esbuild left from
rem an earlier dev run of this folder. Stop only those, drop the stale workspace link
rem (rmdir on a junction removes the link, not the files) and try once more.
echo.
echo [build] node_modules is locked, stopping node/esbuild started from this folder and retrying...
powershell -NoProfile -Command "$d='%ROOT%'; Get-CimInstance Win32_Process -Filter \"Name='node.exe' or Name='esbuild.exe'\" | Where-Object { ($_.CommandLine -and $_.CommandLine.Contains($d)) -or ($_.ExecutablePath -and $_.ExecutablePath.Contains($d)) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"
if exist "node_modules\@seelen-ui\lib" rmdir "node_modules\@seelen-ui\lib" 2>nul
timeout /t 3 /nobreak >nul
call :live npm install && exit /b 0
echo.
echo [build] node_modules is still locked by another program. Close code editors and terminals
echo [build] with this folder open, then run build.bat again. If it keeps failing, delete the
echo [build] node_modules folder by hand or add this folder to the antivirus exclusions.
exit /b 1

:live
rem like :run but the output also shows in the window, long steps look alive
echo.>> "%LOG%"
echo ^> %*>> "%LOG%"
powershell -NoProfile -ExecutionPolicy Bypass -Command "cmd /c '%* 2>&1' | ForEach-Object { $_; Add-Content -LiteralPath '%LOG%' -Value $_ }; exit $LASTEXITCODE"
exit /b %errorlevel%

:run
echo.>> "%LOG%"
echo ^> %*>> "%LOG%"
call %* >> "%LOG%" 2>&1
exit /b %errorlevel%

:fail
echo.
echo ---------------- last lines of build.log ----------------
powershell -NoProfile -Command "Get-Content -Tail 40 -LiteralPath '%LOG%'"
echo ---------------------------------------------------------
echo Build failed. Send build.log if the error above is unclear.
goto :end_fail

:end_fail
echo.
pause
exit /b 1

:end_ok
echo.
pause
exit /b 0
