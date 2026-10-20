@echo off
rem Compile tous les jeux (Rust, cible x86_64-pc-windows-gnu ; editeur de liens = gcc de MSYS2 ucrt64)
set PATH=C:\msys64\ucrt64\bin;%USERPROFILE%\.cargo\bin;%PATH%
cargo build --release
if errorlevel 1 (echo ECHEC& exit /b 1)
copy /y target\release\belote.exe belote.exe >nul
copy /y target\release\421.exe 421.exe >nul
echo OK
