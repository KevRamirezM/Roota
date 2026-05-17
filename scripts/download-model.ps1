# Downloads qwen3-1.7b Q4_K_M (~1.2 GB) into %USERPROFILE%\.roota\models\
$ErrorActionPreference = "Stop"
$Dir = "$env:USERPROFILE\.roota\models"
$Out = Join-Path $Dir "qwen3-1.7b-q4_k_m.gguf"
$Url = "https://huggingface.co/itlwas/Qwen3-1.7B-Q4_K_M-GGUF/resolve/main/qwen3-1.7b-q4_k_m.gguf"

New-Item -ItemType Directory -Force -Path $Dir | Out-Null
if (Test-Path $Out) {
    Write-Host "Model already exists: $Out"
    exit 0
}

Write-Host "Downloading qwen3-1.7b-q4_k_m.gguf (~1.2 GB) to:"
Write-Host "  $Out"
Write-Host ""

# Prefer huggingface-cli when installed (resumable)
$hf = Get-Command huggingface-cli -ErrorAction SilentlyContinue
if ($hf) {
    Write-Host "Using huggingface-cli..."
    huggingface-cli download itlwas/Qwen3-1.7B-Q4_K_M-GGUF qwen3-1.7b-q4_k_m.gguf --local-dir $Dir
    $cliOut = Join-Path $Dir "qwen3-1.7b-q4_k_m.gguf"
    if (Test-Path $cliOut) {
        Write-Host "Done: $cliOut"
        exit 0
    }
}

Write-Host "Using Invoke-WebRequest (may take several minutes)..."
$ProgressPreference = "Continue"
Invoke-WebRequest -Uri $Url -OutFile $Out -UseBasicParsing

if (-not (Test-Path $Out)) {
    throw "Download failed - file not found at $Out"
}

$sizeMb = [math]::Round((Get-Item $Out).Length / 1MB, 1)
Write-Host "Done: $Out ($sizeMb MB)"
