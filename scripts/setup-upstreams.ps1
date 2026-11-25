[CmdletBinding()]
param(
    [string] $MirrorRoot,
    [switch] $CheckOnly
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
$parent = Split-Path -Parent $app
$pins = Get-Content -Raw -LiteralPath (Join-Path $app 'upstreams.json') | ConvertFrom-Json

foreach ($pin in $pins.PSObject.Properties) {
    $name = $pin.Name
    $revision = [string] $pin.Value
    if ($name -notmatch '^auto-(lang|ui|web|down)$' -or $revision -notmatch '^[0-9a-f]{40}$') {
        throw "Invalid upstream pin '$name' in upstreams.json"
    }

    $checkout = Join-Path $parent $name
    $missing = -not (Test-Path -LiteralPath $checkout)
    if ($missing -or (Test-Path -LiteralPath (Join-Path $checkout '.autoscape-bootstrap'))) {
        if ($CheckOnly) {
            throw "Upstream checkout not ready at $checkout. Run setup-upstreams.ps1."
        }
        $source = "https://github.com/auto-stack/$name.git"
        if ($MirrorRoot) {
            $source = (Resolve-Path -LiteralPath (Join-Path $MirrorRoot $name)).Path
        }
        if ($missing) {
            New-Item -Path $checkout -ItemType Directory | Out-Null
            New-Item -Path (Join-Path $checkout '.autoscape-bootstrap') -ItemType File | Out-Null
        }
        if (-not (Test-Path -LiteralPath (Join-Path $checkout '.git'))) {
            Invoke-Git -C $checkout init --quiet
        }
        Invoke-Git -C $checkout config --local core.autocrlf false
        Invoke-Git -C $checkout config --local core.longpaths true
        Invoke-Git -C $checkout fetch --quiet --depth=1 $source $revision
        Invoke-Git -C $checkout checkout --quiet --detach $revision
        Remove-Item -LiteralPath (Join-Path $checkout '.autoscape-bootstrap')
    }

    if (-not (Test-Path -LiteralPath (Join-Path $checkout '.git'))) {
        throw "$checkout must be a Git checkout of $name."
    }
    $actual = Invoke-Git -C $checkout rev-parse HEAD
    if ($actual -ne $revision) {
        throw "$checkout is at $actual, expected $revision. Use a separate parent directory for this checkout."
    }
    if (Invoke-Git -C $checkout status --porcelain --untracked-files=no) {
        throw "$checkout has tracked changes. Use a clean checkout for the pinned upstream."
    }
    if ($name -eq 'auto-ui') {
        $todo = 'examples/ui/013-todo'
        $inputs = Invoke-Git -C $checkout ls-files -- "$todo/README.md" "$todo/pac.at" "$todo/src/*.at"
        foreach ($inputFile in $inputs) {
            $expected = Invoke-Git -C $checkout rev-parse "HEAD:$inputFile"
            $actualBlob = Invoke-Git -C $checkout hash-object --no-filters -- $inputFile
            if ($actualBlob -ne $expected) {
                throw "$inputFile in $checkout differs from the pinned bytes. Set core.autocrlf=false in a fresh checkout."
            }
        }
    }
    Write-Output "$name $actual"
}
