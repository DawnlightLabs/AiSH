param(
    [string]$Binary = "target\debug\aish.exe",
    [string]$OutputPath = "target\aish-routing-acceptance.json",
    [int]$TimeoutSeconds = 120,
    [switch]$ValidateOnly
)

$ErrorActionPreference = "Stop"
$cases = [System.Collections.Generic.List[object]]::new()

function Add-RoutingCase {
    param(
        [string]$Category,
        [string]$InputText,
        [string]$ExpectedRoute
    )
    $cases.Add([pscustomobject]@{
        id = "route-{0:D4}" -f ($cases.Count + 1)
        category = $Category
        input = $InputText
        expected_route = $ExpectedRoute
    })
}

# Slash controls are intercepted before shell resolution or model planning.
$slashControls = @(
    "/version",
    "/update",
    "/model list",
    "/model status",
    "/diagnostics on",
    "/diagnostics off",
    "/context auto",
    "/context clear",
    "/status",
    "/help"
)
for ($index = 0; $index -lt 100; $index++) {
    Add-RoutingCase "slash_control" $slashControls[$index % $slashControls.Count] "slash_command"
}

# Forced literals remain literal even when their executable does not exist.
for ($index = 0; $index -lt 300; $index++) {
    Add-RoutingCase "forced_literal" "//route-fixture-tool-$index --case $index" "forced_literal"
}

# Resolved command heads with ordinary arguments, switches, pipelines, and
# shell-specific syntax must bypass the model. These inputs are classified only.
for ($index = 0; $index -lt 800; $index++) {
    $inputText = switch ($index % 7) {
        0 { "git status --short --untracked-files=all" }
        1 { "Get-ChildItem -Force -Depth $($index % 4)" }
        2 { "cargo test --package aish-provider route_case_$index" }
        3 { "npm run route-check-$index" }
        4 { "rustc --version --verbose" }
        5 { "powershell.exe -NoProfile -Command Write-Output route-$index" }
        6 { "cmd.exe /d /c echo route-$index" }
    }
    Add-RoutingCase "literal_command" $inputText "literal_command"
}

# The command head is valid, so misspelled subcommands and options are still
# literal attempts. Their eventual exit status belongs to recovery, not routing.
for ($index = 0; $index -lt 700; $index++) {
    $inputText = switch ($index % 6) {
        0 { "git pulll-$index" }
        1 { "cargo tset-$index" }
        2 { "npm isntall-$index" }
        3 { "Get-ChildItem -DefinitelyNotAParameter$index" }
        4 { "rustc --definitely-invalid-$index" }
        5 { "powershell.exe -DefinitelyInvalid$index" }
    }
    Add-RoutingCase "mistyped_literal" $inputText "literal_command"
}

# Explicit script paths are command-shaped even when the fixture path is absent.
for ($index = 0; $index -lt 300; $index++) {
    $inputText = if (($index % 2) -eq 0) {
        ".\routing-fixtures\missing-script-$index.ps1 --check"
    } else {
        "./routing-fixtures/missing-script-$index.sh --check"
    }
    Add-RoutingCase "script_path" $inputText "literal_command"
}

# Operators, redirection, and command separators are explicit shell syntax.
# Classification remains literal even when a fixture executable is absent.
for ($index = 0; $index -lt 300; $index++) {
    $inputText = switch ($index % 6) {
        0 { "route-fixture-tool-$index --check | Select-Object -First 1" }
        1 { "route-fixture-tool-$index --left && route-fixture-tool-$index --right" }
        2 { "route-fixture-tool-$index --left || route-fixture-tool-$index --fallback" }
        3 { "route-fixture-tool-$index --first; Write-Output route-$index" }
        4 { ".\routing-fixtures\tool-$index.exe --check" }
        5 { "./routing-fixtures/tool-$index --check" }
    }
    Add-RoutingCase "explicit_shell_syntax" $inputText "literal_command"
}

# Natural-language navigation remains planner-owned, including requests whose
# first word can also be an installed tool name such as `go`.
for ($index = 0; $index -lt 500; $index++) {
    $inputText = switch ($index % 6) {
        0 { "go to the routing fixture folder $index" }
        1 { "navigate to the nearest directory named build-$index" }
        2 { "enter the project folder called sample-$index" }
        3 { "move one directory up from location $index" }
        4 { "open the folder containing manifest-$index.json" }
        5 { "switch into the nested workspace number $index" }
    }
    Add-RoutingCase "nlp_navigation" $inputText "natural_language"
}

for ($index = 0; $index -lt 500; $index++) {
    $inputText = switch ($index % 6) {
        0 { "show hidden files for routing case $index" }
        1 { "find large files in project fixture $index" }
        2 { "check which process uses port $(3000 + $index)" }
        3 { "list the largest Rust files for case $index" }
        4 { "search for TODO comments in fixture $index" }
        5 { "display free disk space for sample $index" }
    }
    Add-RoutingCase "nlp_read_only" $inputText "natural_language"
}

for ($index = 0; $index -lt 400; $index++) {
    $inputText = switch ($index % 5) {
        0 { "create a folder named archive-routing-$index" }
        1 { "create an empty file named note-routing-$index.txt" }
        2 { "rename the routing fixture file number $index" }
        3 { "install the dependency requested by fixture $index" }
        4 { "add the routing fixture directory $index to PATH" }
    }
    Add-RoutingCase "nlp_mutation" $inputText "natural_language"
}

for ($index = 0; $index -lt 350; $index++) {
    $inputText = switch ($index % 5) {
        0 { "why might command fixture $index return access denied?" }
        1 { "explain what a nonzero exit code means for case $index?" }
        2 { "why can a relative path fail in example $index?" }
        3 { "what does git status report in scenario $index?" }
        4 { "explain the previous command without running case $index?" }
    }
    Add-RoutingCase "nlp_explanation" $inputText "natural_language"
}

# Underspecified requests must reach the planner rather than being speculatively
# executed as command text. The planner may then ask a clarification.
for ($index = 0; $index -lt 350; $index++) {
    $inputText = switch ($index % 7) {
        0 { "rename this file for ambiguous case $index" }
        1 { "delete it for ambiguous case $index" }
        2 { "open that folder for ambiguous case $index" }
        3 { "move the selected item for ambiguous case $index" }
        4 { "install the missing tool for ambiguous case $index" }
        5 { "use the other configuration for ambiguous case $index" }
        6 { "fix the previous problem for ambiguous case $index" }
    }
    Add-RoutingCase "nlp_ambiguous" $inputText "natural_language"
}

for ($index = 0; $index -lt 400; $index++) {
    $inputText = switch ($index % 5) {
        0 { "what is the capital of test country number $index?" }
        1 { "write a short haiku about routing case $index" }
        2 { "explain why the sky can look blue in example $index" }
        3 { "summarize the idea of recursion for student $index" }
        4 { "what is two plus $index?" }
    }
    Add-RoutingCase "unrelated_question" $inputText "natural_language"
}

if ($cases.Count -ne 5000) {
    throw "Routing matrix must contain exactly 5000 cases; found $($cases.Count)."
}

$categoryCounts = [ordered]@{}
foreach ($case in $cases) {
    if (-not $categoryCounts.Contains($case.category)) {
        $categoryCounts[$case.category] = 0
    }
    $categoryCounts[$case.category]++
}

if ($ValidateOnly) {
    Write-Host "Routing matrix is valid: $($cases.Count) cases."
    foreach ($entry in $categoryCounts.GetEnumerator()) {
        Write-Host "  $($entry.Key): $($entry.Value)"
    }
    exit 0
}

$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$inputPath = [System.IO.Path]::GetTempFileName()
[System.IO.File]::WriteAllLines(
    $inputPath,
    [string[]]@($cases | ForEach-Object { $_.input }),
    [System.Text.UTF8Encoding]::new($false)
)
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $binaryPath
$startInfo.Arguments = "--route-json-file `"$inputPath`""
$startInfo.UseShellExecute = $false
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
$startInfo.CreateNoWindow = $true

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $startInfo
if (-not $process.Start()) {
    throw "Failed to start $binaryPath."
}
$stdoutTask = $process.StandardOutput.ReadToEndAsync()
$stderrTask = $process.StandardError.ReadToEndAsync()

if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
    $process.Kill($true)
    Remove-Item -LiteralPath $inputPath -Force -ErrorAction SilentlyContinue
    throw "Routing evaluation exceeded the $TimeoutSeconds second timeout."
}
$stdout = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()
Remove-Item -LiteralPath $inputPath -Force -ErrorAction SilentlyContinue
$stopwatch.Stop()

$outputLines = @($stdout -split "\r?\n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
$results = [System.Collections.Generic.List[object]]::new()
for ($index = 0; $index -lt $cases.Count; $index++) {
    $case = $cases[$index]
    $actual = $null
    $parseError = $null
    if ($index -lt $outputLines.Count) {
        try {
            $actual = $outputLines[$index] | ConvertFrom-Json
        } catch {
            $parseError = $_.Exception.Message
        }
    } else {
        $parseError = "No output was returned for this case."
    }
    $actualRoute = if ($null -ne $actual) { [string]$actual.route } else { $null }
    $passed = $null -eq $parseError -and
        $actualRoute -eq $case.expected_route -and
        $actual.executed -eq $false -and
        $actual.model_invoked -eq $false
    $results.Add([pscustomobject]@{
        id = $case.id
        category = $case.category
        input = $case.input
        expected_route = $case.expected_route
        actual_route = $actualRoute
        passed = $passed
        parse_error = $parseError
    })
}

$passedCount = @($results | Where-Object { $_.passed }).Count
$failedCount = $results.Count - $passedCount
$report = [ordered]@{
    schema = "aish.routing-acceptance.v1"
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    execution_policy = "classification_only_no_commands_or_models_executed"
    binary = [System.IO.Path]::GetFileName($binaryPath)
    total_cases = $results.Count
    passed_cases = $passedCount
    failed_cases = $failedCount
    duration_ms = $stopwatch.ElapsedMilliseconds
    process_exit_code = $process.ExitCode
    output_line_count = $outputLines.Count
    category_counts = $categoryCounts
    stderr = $stderr.Trim()
    results = $results
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding utf8
Write-Host "Routing acceptance: $passedCount/$($results.Count) passed in $($stopwatch.ElapsedMilliseconds) ms."
Write-Host "Report: $((Resolve-Path -LiteralPath $OutputPath).Path)"

if ($failedCount -ne 0) {
    Write-Host "First routing mismatches:"
    @($results | Where-Object { -not $_.passed } | Select-Object -First 20) |
        ForEach-Object {
            Write-Host "  $($_.id) [$($_.category)] expected=$($_.expected_route) actual=$($_.actual_route) input=$($_.input)"
        }
}

if ($process.ExitCode -ne 0 -or $failedCount -ne 0 -or $outputLines.Count -ne $cases.Count) {
    exit 1
}
