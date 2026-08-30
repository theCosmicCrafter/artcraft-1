@echo off
setlocal
set PYTHONUTF8=1
set PYTHONPATH=%~dp0;%~dp0backend;%PYTHONPATH%
if not defined ARTCRAFT_STUDIO_DISTRIBUTION set ARTCRAFT_STUDIO_DISTRIBUTION=official
"%~dp0python\python.exe" "%~dp0backend\main.py" %*
