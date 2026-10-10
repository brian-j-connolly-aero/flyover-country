@echo off
rem Flyover Country: double-click to install on first run, then play in your browser.
rem Close this window (or press Ctrl+C) to stop the game.
cd /d "%~dp0"
title Flyover Country

where node >nul 2>nul
if errorlevel 1 goto nonode

if exist "node_modules\vite\package.json" goto play
echo First run: installing the game's tools. This takes a minute or two...
call npm install --no-audit --no-fund
if errorlevel 1 goto failed

:play
echo Starting Flyover Country. Your browser will open at http://localhost:5173
echo Keep this window open while you play.
call npm start
if errorlevel 1 goto failed
goto end

:nonode
echo Flyover Country needs Node.js 20.19 or newer.
echo Install the LTS version from https://nodejs.org and then double-click play.cmd again.
goto failed

:failed
echo.
echo Something went wrong. The message above should say what.
echo If it says port 5173 is already in use, the game is already running:
echo open http://localhost:5173 in your browser.
pause
exit /b 1

:end
