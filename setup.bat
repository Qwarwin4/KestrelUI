@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"

rem Installs everything needed to build KestrelUI. Safe to run again.

where winget >nul 2>&1
if errorlevel 1 (
  echo winget not found. Install "App Installer" from Microsoft Store and run this again.
  exit /b 1
)

set "WG=winget install -e --silent --accept-package-agreements --accept-source-agreements --id"

echo [1/6] Visual Studio Build Tools 2022 (MSVC + Windows SDK)
%WG% Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
call :ensure_vctools || exit /b 1

echo [2/6] Rust
%WG% Rustlang.Rustup

echo [3/6] Node.js LTS
%WG% OpenJS.NodeJS.LTS

echo [4/6] Deno
%WG% DenoLand.Deno

echo [5/6] Git and WebView2
%WG% Git.Git
%WG% Microsoft.EdgeWebView2Runtime

echo [6/6] Rust toolchain from rust-toolchain.toml
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
rustup show active-toolchain >nul 2>&1
if errorlevel 1 (
  echo rustup is not in PATH yet. Open a new terminal and run setup.bat again.
  exit /b 1
)
rustup show active-toolchain
rustup target add x86_64-pc-windows-msvc

echo.
echo Done. Open a NEW terminal and run build.bat
exit /b 0

:ensure_vctools
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo Visual Studio Installer was not installed. Check the winget output above.
  exit /b 1
)
set "VSPATH="
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSPATH=%%i"
if defined VSPATH (
  echo MSVC found: !VSPATH!
  exit /b 0
)

rem Build Tools were already there but without the C++ workload: add it
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -property installationPath`) do set "VSPATH=%%i"
if not defined VSPATH (
  echo No Visual Studio installation found.
  exit /b 1
)
echo Adding C++ workload to !VSPATH!
"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\setup.exe" modify --installPath "!VSPATH!" --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart --wait
exit /b 0
