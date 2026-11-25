[CmdletBinding()]
param(
    [string] $MirrorRoot,
    [switch] $Offline
)

$ErrorActionPreference = 'Stop'

function Invoke-Git {
    $result = & git @args
    if ($LASTEXITCODE -ne 0) {
        throw "git $($args -join ' ') exited with $LASTEXITCODE"
    }
    $result
}

$app = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$revision = Invoke-Git -C $app rev-parse HEAD
if (Invoke-Git -C $app status --porcelain --untracked-files=no) {
    throw 'Commit the AutoScape source changes before building the AutoOS app.'
}

$target = Join-Path $app 'target'
# The standalone manifest consumes sibling engine sources. Keep its frozen
# source group outside the AutoOS app registry and available to the executable.
$group = Join-Path $target "auto-os/$($revision.Substring(0, 12))"
$buildApp = Join-Path $group 'auto-scape'
if (-not (Test-Path -LiteralPath $buildApp)) {
    New-Item -Path $group -ItemType Directory -Force | Out-Null
    Invoke-Git clone --quiet --no-hardlinks --no-checkout -- $app $buildApp
    Invoke-Git -C $buildApp config --local core.autocrlf false
    Invoke-Git -C $buildApp config --local core.longpaths true
    Invoke-Git -C $buildApp checkout --quiet --detach $revision
}
if ((Invoke-Git -C $buildApp rev-parse HEAD) -ne $revision -or
    (Invoke-Git -C $buildApp status --porcelain --untracked-files=no)) {
    throw "The build checkout at $buildApp must be clean and at $revision."
}

$setupArgs = @{}
if ($MirrorRoot) {
    $setupArgs.MirrorRoot = (Resolve-Path -LiteralPath $MirrorRoot).Path
}
& (Join-Path $buildApp 'scripts/setup-upstreams.ps1') @setupArgs

$cargoArgs = @('build', '--locked', '--target-dir', $target)
if ($Offline) { $cargoArgs += '--offline' }
Push-Location -LiteralPath $buildApp
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "AutoScape build exited with $LASTEXITCODE" }
} finally {
    Pop-Location
}
Get-Item -LiteralPath (Join-Path $target 'debug/auto-scape.exe')
