@echo off
rem Chronos demo: build, open the browser, serve the workshop on localhost.
rem Usage: demo.cmd [port]
setlocal
cd /d "%~dp0"
set "PORT=%~1"
if "%PORT%"=="" set "PORT=8785"

cargo build --release -p chronos-app || exit /b 1
start "" "http://127.0.0.1:%PORT%/"
"%~dp0target\release\chronos-app.exe" serve --port %PORT%
