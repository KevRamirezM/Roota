$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Bin = Join-Path $Root "bin\llama-server.exe"
$Model = "$env:USERPROFILE\.roota\models\qwen3-1.7b-q4_k_m.gguf"
$Threads = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
if (-not $Threads -or $Threads -lt 1) { $Threads = 4 }
$Context = 1024
if (-not (Test-Path $Bin)) {
    throw "Missing $Bin - download llama-server from https://github.com/ggml-org/llama.cpp/releases"
}
if (-not (Test-Path $Model)) {
    throw "Missing $Model - run scripts/download-model.ps1"
}
Write-Host "llama-server: threads=$Threads context=$Context model=$Model"
& $Bin -m $Model -t $Threads --batch-size 512 -c $Context --host 127.0.0.1 --port 8080
