@echo off
REM ============================================================================
REM  qsi-pulse-reader R module installer (Windows batch version)
REM
REM  Steps:
REM    1. Copy canonical rust-core crate (without target/) into R src tree.
REM    2. Install the R package with remotes::install_local (fallback to R CMD INSTALL).
REM
REM  Usage (from any directory after cloning repository):
REM    call R\qsi.pulse.reader\tools\install_r_module.bat
REM
REM  Optional environment variables (set before calling or with `set VAR=...`):
REM    R_CMD         (default: R)      - R executable name
REM    FORCE         (default: true)   - passed to remotes::install_local(force=...)
REM    KEEP_RUST_CORE (default:false)  - if true, keep staged copy after install
REM    VERBOSE       (default: false)  - if true, do not pass -q to R
REM ============================================================================

setlocal ENABLEDELAYEDEXPANSION

REM ---- Defaults ----
if not defined R_CMD set "R_CMD=R"
if not defined FORCE set "FORCE=true"
if not defined KEEP_RUST_CORE set "KEEP_RUST_CORE=false"
if not defined VERBOSE set "VERBOSE=false"

REM ---- Resolve paths ----
set "SCRIPT_DIR=%~dp0"
REM SCRIPT_DIR ends with backslash; navigate to repo root ( ..\.. from tools dir )
pushd "%SCRIPT_DIR%..\..\.." >NUL 2>&1
set "ROOT_DIR=%CD%"
popd >NUL 2>&1
set "PKG_DIR=%ROOT_DIR%\R\qsi.pulse.reader"
set "RUST_SRC=%ROOT_DIR%\rust-core"
set "RUST_DST=%PKG_DIR%\src\rust\rust-core"

call :log Root: %ROOT_DIR%

IF NOT EXIST "%RUST_SRC%\src" (
  call :log ERROR: Missing %RUST_SRC%\src (ensure full repository clone)
  exit /b 2
)

REM ---- Stage rust-core ----
call :log Staging rust-core -> %RUST_DST%
IF EXIST "%RUST_DST%" rmdir /S /Q "%RUST_DST%"
mkdir "%RUST_DST%" || (call :log ERROR: Could not create %RUST_DST% & exit /b 2)

xcopy "%RUST_SRC%\src" "%RUST_DST%\src" /E /I /Q /Y >NUL || (call :log ERROR copying src && exit /b 2)
copy /Y "%RUST_SRC%\Cargo.toml" "%RUST_DST%" >NUL || (call :log ERROR copying Cargo.toml && exit /b 2)
IF EXIST "%RUST_SRC%\Cargo.lock" copy /Y "%RUST_SRC%\Cargo.lock" "%RUST_DST%" >NUL
IF EXIST "%RUST_SRC%\LICENSE.rst" copy /Y "%RUST_SRC%\LICENSE.rst" "%RUST_DST%" >NUL

REM ---- Install ----
set "R_FLAGS=-q"
if /I "%VERBOSE%"=="true" set "R_FLAGS="
call :log Installing R package (FORCE=%FORCE%, VERBOSE=%VERBOSE%)
set "INSTALL_EXPR=if(!requireNamespace('remotes', quietly=TRUE)) install.packages('remotes'); remotes::install_local('%PKG_DIR:/=/%', force=%FORCE%)"

"%R_CMD%" %R_FLAGS% -e "%INSTALL_EXPR%"
set "RC=%ERRORLEVEL%"
if NOT "%RC%"=="0" (
  call :log remotes::install_local failed (code %RC%); falling back to R CMD INSTALL
  pushd "%PKG_DIR%"
  "%R_CMD%" CMD INSTALL .
  set "RC=%ERRORLEVEL%"
  popd
  if NOT "%RC%"=="0" (
    call :log Fallback install failed (code %RC%)
    exit /b 3
  )
)

REM ---- Cleanup ----
if /I NOT "%KEEP_RUST_CORE%"=="true" (
  call :log Cleaning staged rust-core (KEEP_RUST_CORE=true to retain)
  rmdir /S /Q "%RUST_DST%" 2>NUL
) else (
  call :log Leaving staged rust-core in place (KEEP_RUST_CORE=true)
)

call :log Done.
exit /b 0

:log
>&2 echo [qsi-pulse-reader][install] %*
exit /b 0
