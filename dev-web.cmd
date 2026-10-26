@echo off
setlocal
cd /d "%~dp0"

where corepack >nul 2>&1
if errorlevel 1 (
  echo corepack was not found. Node.js 22.23.4 is required.
  pause
  exit /b 1
)

if not exist "node_modules\" (
  echo node_modules is missing. Running corepack yarn install --immutable
  corepack yarn install --immutable
  if errorlevel 1 (
    echo Install failed.
    pause
    exit /b 1
  )
)

echo Public web: http://127.0.0.1:4321/
echo Press Ctrl+C to stop.
corepack yarn workspace @datagrizzly/web dev --host 127.0.0.1 --port 4321
set EXITCODE=%ERRORLEVEL%
if not "%EXITCODE%"=="0" pause
exit /b %EXITCODE%
