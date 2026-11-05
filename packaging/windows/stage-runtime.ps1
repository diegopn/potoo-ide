param(
    [string]$Prefix = "C:\msys64\ucrt64",
    [string]$Target = "target\x86_64-pc-windows-gnu\release\potoo-ide.exe",
    [string]$StageRoot = "target\packages\windows\stage"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$prefixRoot = (Resolve-Path $Prefix).Path
$stagePath = Join-Path $repoRoot $StageRoot
$binaryPath = Join-Path $repoRoot $Target
$binPath = Join-Path $stagePath "bin"

function Copy-Tree {
    param([string]$Source, [string]$Destination)

    if (-not (Test-Path -LiteralPath $Source)) {
        throw "Required runtime directory not found: $Source"
    }

    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Copy-Item -Path (Join-Path $Source "*") -Destination $Destination -Recurse -Force
}

function Copy-TreeIfPresent {
    param([string]$Source, [string]$Destination)

    if (Test-Path -LiteralPath $Source) {
        Copy-Tree -Source $Source -Destination $Destination
    }
}

if (-not (Test-Path -LiteralPath $binaryPath)) {
    throw "Release executable not found: $binaryPath"
}

if (Test-Path -LiteralPath $stagePath) {
    Remove-Item -LiteralPath $stagePath -Recurse -Force
}
New-Item -ItemType Directory -Path $binPath -Force | Out-Null

Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $binPath "potoo-ide.exe")
Copy-Item -Path (Join-Path $prefixRoot "bin\*.dll") -Destination $binPath -Force
Copy-Item -LiteralPath (Join-Path $prefixRoot "bin\gdk-pixbuf-query-loaders.exe") -Destination $binPath
$gstScanner = Join-Path $prefixRoot "libexec\gstreamer-1.0\gst-plugin-scanner.exe"
if (-not (Test-Path -LiteralPath $gstScanner)) {
    $gstScanner = Join-Path $prefixRoot "bin\gst-plugin-scanner.exe"
}
if (-not (Test-Path -LiteralPath $gstScanner)) {
    throw "Required GStreamer plugin scanner not found."
}
Copy-Item -LiteralPath $gstScanner -Destination (Join-Path $binPath "gst-plugin-scanner.exe")

$runtimeDirectories = @(
    @{ Source = "share\glib-2.0"; Destination = "share\glib-2.0" },
    @{ Source = "share\gtk-4.0"; Destination = "share\gtk-4.0" },
    @{ Source = "share\gtksourceview-5"; Destination = "share\gtksourceview-5" },
    @{ Source = "share\icons\Adwaita"; Destination = "share\icons\Adwaita" },
    @{ Source = "share\icons\hicolor"; Destination = "share\icons\hicolor" },
    @{ Source = "lib\gdk-pixbuf-2.0"; Destination = "lib\gdk-pixbuf-2.0" },
    @{ Source = "lib\gio\modules"; Destination = "lib\gio\modules" },
    @{ Source = "lib\gtk-4.0"; Destination = "lib\gtk-4.0" },
    @{ Source = "lib\gstreamer-1.0"; Destination = "lib\gstreamer-1.0" },
    @{ Source = "share\gstreamer-1.0"; Destination = "share\gstreamer-1.0" },
    @{ Source = "etc\fonts"; Destination = "etc\fonts" },
    @{ Source = "share\fontconfig"; Destination = "share\fontconfig" },
    @{ Source = "share\licenses"; Destination = "share\licenses" }
)

foreach ($directory in $runtimeDirectories) {
    Copy-TreeIfPresent `
        -Source (Join-Path $prefixRoot $directory.Source) `
        -Destination (Join-Path $stagePath $directory.Destination)
}

Copy-Tree `
    -Source (Join-Path $repoRoot "data\icons\hicolor") `
    -Destination (Join-Path $stagePath "share\icons\hicolor")
Copy-Tree `
    -Source (Join-Path $repoRoot "data\fonts") `
    -Destination (Join-Path $stagePath "share\potoo-ide\fonts")
Copy-Item `
    -LiteralPath (Join-Path $repoRoot "data\icons\windows\io.github.diegopn.potooide.ico") `
    -Destination (Join-Path $stagePath "potoo-ide.ico")
Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination $stagePath

$settingsPath = Join-Path $stagePath "etc\gtk-4.0\settings.ini"
if (-not (Test-Path -LiteralPath $settingsPath)) {
    New-Item -ItemType Directory -Path (Split-Path $settingsPath) -Force | Out-Null
    @"
[Settings]
gtk-theme-name=Adwaita
"@ | Set-Content -LiteralPath $settingsPath -Encoding utf8
}

$schemasPath = Join-Path $stagePath "share\glib-2.0\schemas"
$compileSchemas = Join-Path $prefixRoot "bin\glib-compile-schemas.exe"
if ((Test-Path -LiteralPath $compileSchemas) -and (Test-Path -LiteralPath $schemasPath)) {
    & $compileSchemas $schemasPath
    if ($LASTEXITCODE -ne 0) {
        throw "glib-compile-schemas failed for the staged runtime."
    }
}

Write-Host "Staged GTK runtime in $stagePath"
