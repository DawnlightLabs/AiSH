param(
    [string]$Binary = "target\debug\aish.exe",
    [string]$ModelsDirectory = "models",
    [string]$Model = "qwen2-5-coder-1-5b-instruct-q6-k",
    [string]$OutputPath = "target\aish-dynamic-sandbox-acceptance.json",
    [int]$SkipCases = 0,
    [int]$MaxCases = 0,
    [switch]$ValidateOnly,
    [switch]$KeepFixture
)

$ErrorActionPreference = "Stop"
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$suffix = [Guid]::NewGuid().ToString("N").Substring(0, 8)
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) "aish-dynamic-$suffix"
$simple = "Orbit-$suffix"
$spaced = "Work Area $suffix"
$nested = "Nested-$suffix"
$unicode = "Cafe-$([char]0x00E9)-$suffix"
$ambiguous = "Echo-$suffix"
$archive = "Archive $suffix"
$emptyFile = "Empty Note $suffix.txt"
$renamedFile = "Final Name $suffix.txt"
$copiedCargo = "Cargo Copy $suffix.toml"
$movedFile = "Move Source $suffix.txt"
$resultFile = "Result $suffix.txt"
$batchFolder = "Batch $suffix"
$batchFile = Join-Path $batchFolder "Created Together $suffix.txt"
$disposableFile = "Disposable $suffix.tmp"
$disposableFolder = "Disposable Folder $suffix"

function Add-DynamicCase {
    param(
        [string]$Category,
        [string]$Prompt,
        [string[]]$ExpectedActions,
        [string]$Mode = "plan",
        [bool]$Execute = $false,
        [string]$AllowedCommandRegex = "",
        [string]$ExpectedOutputRegex = "",
        [string]$ExpectedTargetRegex = "",
        [string[]]$ExpectedPaths = @(),
        [string[]]$MissingPaths = @(),
        [int[]]$ExpectedExitCodes = @(0),
        [string]$ContextJson = "",
        [Nullable[int]]$RecoveryExitCode = $null,
        [string]$RecoveryStderr = ""
    )
    $script:cases.Add([pscustomobject]@{
        category = $Category
        prompt = $Prompt
        expected_actions = $ExpectedActions
        mode = $Mode
        execute = $Execute
        allowed_command_regex = $AllowedCommandRegex
        expected_output_regex = $ExpectedOutputRegex
        expected_target_regex = $ExpectedTargetRegex
        expected_paths = $ExpectedPaths
        missing_paths = $MissingPaths
        expected_exit_codes = $ExpectedExitCodes
        context_json = $ContextJson
        recovery_exit_code = $RecoveryExitCode
        recovery_stderr = $RecoveryStderr
    })
}

function Redact-Text {
    param([string]$Text)
    if ($null -eq $Text) {
        return $null
    }
    return $Text.Replace($fixtureRoot, "<fixture-root>").Replace($env:USERPROFILE, "<user-profile>")
}

function Remove-DynamicFixture {
    if ($KeepFixture -or -not (Test-Path -LiteralPath $fixtureRoot)) {
        return
    }
    $resolvedFixture = [System.IO.Path]::GetFullPath($fixtureRoot)
    $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if (-not $resolvedFixture.StartsWith($resolvedTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean a fixture outside the system temporary directory."
    }
    Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
}

function Test-CommandScope {
    param(
        [string]$Command,
        [string]$AllowedCommandRegex
    )
    $failures = [System.Collections.Generic.List[string]]::new()
    if ([string]::IsNullOrWhiteSpace($Command)) {
        $failures.Add("The plan did not contain an executable command.")
        return @($failures)
    }
    if (
        -not [string]::IsNullOrWhiteSpace($AllowedCommandRegex) -and
        -not [regex]::IsMatch($Command, $AllowedCommandRegex, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    ) {
        $failures.Add("The command did not match the scenario's allowed operation.")
    }
    if ($Command.Contains("..")) {
        $failures.Add("Parent-path traversal is not allowed in executable sandbox cases.")
    }
    if ($Command -match '(?i)https?://|\$env:|%[A-Za-z_][A-Za-z0-9_]*%|(?:^|\s)~(?:[/\\]|\s|$)') {
        $failures.Add("Environment, home, or network expansion is not allowed in executable sandbox cases.")
    }
    $forbidden = '(?i)\b(?:format|shutdown|restart-computer|set-executionpolicy|setx|winget|choco|scoop|apt|apt-get|brew|sudo|taskkill|stop-process|stop-service|start-service|start-process|invoke-expression|iex|reg|icacls|takeown)\b'
    if ($Command -match $forbidden) {
        $failures.Add("A system-management command was blocked by the sandbox harness.")
    }
    if ($Command -match '(?i)(?:^|[|;&]\s*)(?:cmd(?:\.exe)?|start)\b') {
        $failures.Add("Detached or nested command-shell launchers are not allowed in executable sandbox cases.")
    }
    $absolutePattern = '(?i)(?:\\\\\?\\)?[A-Z]:\\[^''"`|;)}]+'
    foreach ($match in [regex]::Matches($Command, $absolutePattern)) {
        $candidate = $match.Value.Trim()
        if ($candidate.StartsWith("\\?\")) {
            $candidate = $candidate.Substring(4)
        }
        if (-not $candidate.StartsWith($fixtureRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            $failures.Add("The command referenced an absolute path outside the sandbox.")
        }
    }
    return @($failures | Select-Object -Unique)
}

function Invoke-BoundedCommand {
    param([string]$Command)
    $scriptPath = Join-Path $fixtureRoot ".aish-dynamic-command"
    if ($env:OS -eq "Windows_NT") {
        $scriptPath += ".ps1"
$scriptBody = @"
$Command
`$aishCommandSucceeded = `$?
if (`$null -ne `$LASTEXITCODE) { exit `$LASTEXITCODE }
if (-not `$aishCommandSucceeded) { exit 1 }
"@
        [System.IO.File]::WriteAllText(
            $scriptPath,
            $scriptBody,
            [System.Text.UTF8Encoding]::new($false)
        )
        $fileName = "powershell.exe"
        $arguments = "-NoLogo -NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`""
    } else {
        $scriptPath += ".sh"
        [System.IO.File]::WriteAllText(
            $scriptPath,
            "#!/bin/sh`nset -u`n$Command`n",
            [System.Text.UTF8Encoding]::new($false)
        )
        $fileName = "/bin/sh"
        $arguments = "`"$scriptPath`""
    }
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $fileName
    $start.Arguments = $arguments
    $start.WorkingDirectory = $fixtureRoot
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    if (-not $process.Start()) {
        throw "Failed to start the bounded sandbox command."
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(10000)) {
        try {
            $process.Kill()
        } catch {
        }
        return [pscustomobject]@{
            exit_code = $null
            stdout = $stdoutTask.GetAwaiter().GetResult()
            stderr = "Sandbox command exceeded the 10 second limit."
            timed_out = $true
        }
    }
    return [pscustomobject]@{
        exit_code = $process.ExitCode
        stdout = $stdoutTask.GetAwaiter().GetResult()
        stderr = $stderrTask.GetAwaiter().GetResult()
        timed_out = $false
    }
}

function Test-Postconditions {
    param(
        [object]$Case,
        [object]$Execution
    )
    $failures = [System.Collections.Generic.List[string]]::new()
    foreach ($relative in $Case.expected_paths) {
        if (-not (Test-Path -LiteralPath (Join-Path $fixtureRoot $relative))) {
            $failures.Add("Expected path was not created: $relative")
        }
    }
    foreach ($relative in $Case.missing_paths) {
        if (Test-Path -LiteralPath (Join-Path $fixtureRoot $relative)) {
            $failures.Add("Expected path still exists: $relative")
        }
    }
    if (
        $null -ne $Execution -and
        -not [string]::IsNullOrWhiteSpace($Case.expected_output_regex) -and
        -not [regex]::IsMatch(
            "$($Execution.stdout)`n$($Execution.stderr)",
            $Case.expected_output_regex,
            [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
        )
    ) {
        $failures.Add("Execution output did not match the expected result.")
    }
    return @($failures)
}

$cases = [System.Collections.Generic.List[object]]::new()
$folderSizeContext = @{
    session_commands = @(
        @{
            intent = "find the 10 largest folders and subfolders up to 3 levels"
            command = "Get-ChildItem -Recurse -Depth 3 -Directory | Select-Object -First 10"
            status = "success"
            reason = "fixture"
        }
    )
    session_turns = @(
        @{
            request = "find the 10 largest folders and subfolders up to 3 levels"
            outcome = "listed the ten largest directories"
        }
    )
} | ConvertTo-Json -Compress -Depth 6
$websiteContext = @{
    session_turns = @(
        @{
            request = "run this React website"
            outcome = "Which existing directory contains the website?"
        }
    )
} | ConvertTo-Json -Compress -Depth 6
$namedSearchContext = @{
    session_turns = @(
        @{
            request = "where is the folder named $nested under this directory"
            outcome = "Should subdirectories be included in the search?"
        }
    )
} | ConvertTo-Json -Compress -Depth 6

Add-DynamicCase observation "show the current directory" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-Location|pwd)' -ExpectedOutputRegex ([regex]::Escape("aish-dynamic-$suffix"))
Add-DynamicCase observation "list child directories only" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-ChildItem.*-Directory|find .*type d)' -ExpectedOutputRegex ([regex]::Escape($simple))
Add-DynamicCase observation "show hidden files here" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-ChildItem.*-Force|ls\s+-[^ ]*a)' -ExpectedOutputRegex 'hidden-marker'
Add-DynamicCase observation "find every package.json below here" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-ChildItem.*-Recurse|find )' -ExpectedOutputRegex ([regex]::Escape($simple))
Add-DynamicCase observation "count files in this directory" @("shell_command") -Execute $true -AllowedCommandRegex '(Measure-Object|wc\s+-l)' -ExpectedOutputRegex '\b[0-9]+\b'
Add-DynamicCase observation "find the largest files in this project" @("shell_command") -Execute $true -AllowedCommandRegex '(Length|stat |find )' -ExpectedOutputRegex 'large-marker'
Add-DynamicCase observation "search recursively for the word TODO_DYNAMIC" @("shell_command") -Execute $true -AllowedCommandRegex '(Select-String|rg |grep )' -ExpectedOutputRegex 'TODO_DYNAMIC'
Add-DynamicCase observation "test whether package.json exists" @("shell_command") -Execute $true -AllowedCommandRegex '(Test-Path|test\s+-[ef])' -ExpectedOutputRegex 'true'
Add-DynamicCase observation "where is the folder named $nested under this directory, include all subdirectories" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-ChildItem.*-Recurse|find )' -ExpectedOutputRegex ([regex]::Escape($nested))
Add-DynamicCase navigation "go to $simple" @("change_directory") -ExpectedTargetRegex ([regex]::Escape($simple))
Add-DynamicCase navigation "go to '$spaced'" @("change_directory") -ExpectedTargetRegex ([regex]::Escape($spaced))
Add-DynamicCase navigation "enter the Unicode directory '$unicode'" @("change_directory") -ExpectedTargetRegex ([regex]::Escape($unicode))
Add-DynamicCase navigation "go to the folder called $ambiguous under this test directory" @("fallback")
Add-DynamicCase mutation "create a folder named $archive" @("approval_required") -Execute $true -AllowedCommandRegex '(New-Item|mkdir)' -ExpectedPaths @($archive)
Add-DynamicCase mutation "create an empty file named $emptyFile" @("approval_required") -Execute $true -AllowedCommandRegex '(New-Item|touch)' -ExpectedPaths @($emptyFile)
Add-DynamicCase mutation "rename source-$suffix.txt to $renamedFile" @("approval_required") -Execute $true -AllowedCommandRegex '(Rename-Item|mv )' -ExpectedPaths @($renamedFile) -MissingPaths @("source-$suffix.txt")
Add-DynamicCase mutation "copy Cargo.toml to $copiedCargo" @("approval_required") -Execute $true -AllowedCommandRegex '(Copy-Item|cp )' -ExpectedPaths @($copiedCargo)
Add-DynamicCase mutation "move $movedFile into $simple" @("approval_required") -Execute $true -AllowedCommandRegex '(Move-Item|mv )' -ExpectedPaths @((Join-Path $simple $movedFile)) -MissingPaths @($movedFile)
Add-DynamicCase mutation "write hello dynamic sandbox into $resultFile" @("approval_required") -Execute $true -AllowedCommandRegex '(Set-Content|printf |echo )' -ExpectedPaths @($resultFile)
Add-DynamicCase mutation "append second dynamic line to $resultFile" @("approval_required") -Execute $true -AllowedCommandRegex '(Add-Content|printf |echo )' -ExpectedPaths @($resultFile)
Add-DynamicCase mutation "create a folder named $batchFolder and then create an empty file named Created Together $suffix.txt inside it" @("approval_required") -Execute $true -AllowedCommandRegex '(New-Item|mkdir).*(New-Item|touch)' -ExpectedPaths @($batchFolder, $batchFile)
Add-DynamicCase mutation "delete $disposableFile" @("approval_required") -Execute $true -AllowedCommandRegex '(Remove-Item|rm )' -MissingPaths @($disposableFile)
Add-DynamicCase mutation "delete the folder named $disposableFolder" @("approval_required") -Execute $true -AllowedCommandRegex '(Remove-Item|rm |rmdir )' -MissingPaths @($disposableFolder)
Add-DynamicCase script "run the PowerShell script marker-task.ps1 in this folder" @("approval_required", "shell_command") -Execute $true -AllowedCommandRegex '((powershell|pwsh).*)?marker-task\.ps1' -ExpectedOutputRegex 'DYNAMIC_SCRIPT_OK'
Add-DynamicCase script "run args-task.ps1 with alpha and beta" @("approval_required", "shell_command") -Execute $true -AllowedCommandRegex '((powershell|pwsh).*)?args-task\.ps1.*alpha.*beta' -ExpectedOutputRegex 'alpha.*beta'
Add-DynamicCase script "run failing-task.ps1 and show its exit status" @("approval_required", "shell_command") -Execute $true -AllowedCommandRegex '((powershell|pwsh).*)?failing-task\.ps1' -ExpectedOutputRegex 'EXPECTED_DYNAMIC_FAILURE' -ExpectedExitCodes @(7)
Add-DynamicCase terminal "start a bounded PowerShell task that prints TERMINAL_TASK_OK and exits" @("approval_required", "shell_command") -Execute $true -AllowedCommandRegex '(powershell|pwsh).*TERMINAL_TASK_OK' -ExpectedOutputRegex 'TERMINAL_TASK_OK'
Add-DynamicCase observation "show the current PowerShell version" @("shell_command") -Execute $true -AllowedCommandRegex '(PSVersionTable|pwsh.*version)' -ExpectedOutputRegex '(Major\s+Minor|\d+\.\d+)'
Add-DynamicCase observation "show listening TCP ports" @("shell_command") -Execute $true -AllowedCommandRegex '(Get-NetTCPConnection|netstat|ss |lsof)' -ExpectedExitCodes @(0, 1)
Add-DynamicCase follow_up "i need the sizes in gb" @("shell_command", "approval_required") -AllowedCommandRegex '(SizeGB|GB|du )' -ContextJson $folderSizeContext
Add-DynamicCase follow_up "only show the top five" @("shell_command", "approval_required") -AllowedCommandRegex '(-First\s+5|head\s+-n\s+5)' -ContextJson $folderSizeContext
Add-DynamicCase follow_up "run this website" @("approval_required") -AllowedCommandRegex '(npm\s+run\s+dev|pnpm\s+run\s+dev|yarn\s+dev|bun\s+run\s+dev)' -ContextJson $websiteContext
Add-DynamicCase follow_up "use the current folder" @("approval_required") -AllowedCommandRegex '(npm\s+run\s+dev|pnpm\s+run\s+dev|yarn\s+dev|bun\s+run\s+dev)' -ContextJson $websiteContext
Add-DynamicCase follow_up "include all subdirectories" @("shell_command") -AllowedCommandRegex ([regex]::Escape($nested)) -ContextJson $namedSearchContext
Add-DynamicCase ambiguous "rename this file" @("fallback")
Add-DynamicCase ambiguous "delete the old one" @("fallback")
Add-DynamicCase ambiguous "run the script" @("fallback")
Add-DynamicCase missing "go to definitely-missing-$suffix" @("fallback")
Add-DynamicCase recovery "gti status" @("shell_command", "fallback") -Mode recovery -RecoveryExitCode 127 -RecoveryStderr "gti is not recognized"
Add-DynamicCase routing "git status" @("literal_command") -Mode route
Add-DynamicCase routing "git stats" @("literal_command") -Mode route
Add-DynamicCase routing "pull the latest updates" @("natural_language") -Mode route

if ($cases.Count -ne 42) {
    throw "Dynamic acceptance suite must contain 42 cases; found $($cases.Count)."
}
if ($ValidateOnly) {
    Write-Host "Dynamic sandbox suite is valid: $($cases.Count) cases."
    return
}
if ($SkipCases -gt 0) {
    $cases = @($cases | Select-Object -Skip $SkipCases)
}
if ($MaxCases -gt 0) {
    $cases = @($cases | Select-Object -First $MaxCases)
}

$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$modelsPath = (Resolve-Path -LiteralPath $ModelsDirectory).Path
$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutputPath))
$contextFile = Join-Path $fixtureRoot ".aish-dynamic-context.json"
$hadModelsDirectory = Test-Path Env:AISH_MODELS_DIR
$previousModelsDirectory = $env:AISH_MODELS_DIR
$env:AISH_MODELS_DIR = $modelsPath
$results = [System.Collections.Generic.List[object]]::new()

New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $simple) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $spaced) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $unicode) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot $simple) $nested) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot "left") $ambiguous) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot "right") $ambiguous) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $disposableFolder) | Out-Null
Set-Content -LiteralPath (Join-Path $fixtureRoot "package.json") -Value '{"private":true,"scripts":{"dev":"vite"}}' -NoNewline
Set-Content -LiteralPath (Join-Path (Join-Path $fixtureRoot $simple) "package.json") -Value '{"private":true}' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "Cargo.toml") -Value '[workspace]' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "source-$suffix.txt") -Value 'rename source' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot $movedFile) -Value 'move source' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot $disposableFile) -Value 'delete source' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "todo-marker.txt") -Value 'TODO_DYNAMIC fixture marker' -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "hidden-marker.txt") -Value 'hidden' -NoNewline
if ($env:OS -eq "Windows_NT") {
    (Get-Item -LiteralPath (Join-Path $fixtureRoot "hidden-marker.txt")).Attributes = [System.IO.FileAttributes]::Hidden
}
[System.IO.File]::WriteAllBytes((Join-Path $fixtureRoot "large-marker.bin"), [byte[]]::new(1024 * 1024))
Set-Content -LiteralPath (Join-Path $fixtureRoot "marker-task.ps1") -Value "Write-Output 'DYNAMIC_SCRIPT_OK'" -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "args-task.ps1") -Value "param([string]`$First,[string]`$Second); Write-Output `"`$First `$Second`"" -NoNewline
Set-Content -LiteralPath (Join-Path $fixtureRoot "failing-task.ps1") -Value "Write-Error 'EXPECTED_DYNAMIC_FAILURE'; exit 7" -NoNewline

Push-Location $fixtureRoot
try {
    foreach ($case in $cases) {
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        $raw = @()
        $plan = $null
        $processExit = $null
        $failure = $null
        $execution = $null
        $scopeFailures = @()
        $postconditionFailures = @()
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            if ($case.mode -eq "route") {
                $raw = @(& $binaryPath --route-json $case.prompt 2>&1)
            } else {
                $arguments = if ($case.mode -eq "recovery") {
                    @(
                        "--recover-json",
                        $case.prompt,
                        "--exit-code",
                        [string]$case.recovery_exit_code,
                        "--stderr",
                        $case.recovery_stderr,
                        "--model",
                        $Model,
                        "--diagnostics"
                    )
                } else {
                    @("--plan-json", $case.prompt, "--model", $Model, "--diagnostics")
                }
                if (-not [string]::IsNullOrWhiteSpace($case.context_json)) {
                    [System.IO.File]::WriteAllText(
                        $contextFile,
                        $case.context_json,
                        [System.Text.UTF8Encoding]::new($false)
                    )
                    $arguments += @("--context-json-file", $contextFile)
                }
                $raw = @(& $binaryPath @arguments 2>&1)
            }
            $processExit = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        $watch.Stop()
        $jsonLine = $raw | Where-Object { $_.ToString().TrimStart().StartsWith("{") } | Select-Object -Last 1
        if ($null -eq $jsonLine) {
            $failure = ($raw | ForEach-Object { $_.ToString() }) -join "`n"
        } else {
            try {
                $plan = $jsonLine.ToString() | ConvertFrom-Json
            } catch {
                $failure = $_.Exception.Message
            }
        }
        $action = if ($null -eq $plan) {
            "process_failure"
        } elseif ($case.mode -eq "route") {
            [string]$plan.route
        } else {
            [string]$plan.action
        }
        $passed = $case.expected_actions -contains $action
        $command = if ($null -ne $plan -and $null -ne $plan.command) {
            [string]$plan.command
        } else {
            ""
        }
        if (
            $passed -and
            -not [string]::IsNullOrWhiteSpace($case.allowed_command_regex) -and
            -not [regex]::IsMatch($command, $case.allowed_command_regex, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        ) {
            $passed = $false
            $failure = "Planned command did not preserve the scenario's required operation."
        }
        if (
            $passed -and
            -not [string]::IsNullOrWhiteSpace($case.expected_target_regex) -and
            -not [regex]::IsMatch([string]$plan.target, $case.expected_target_regex, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        ) {
            $passed = $false
            $failure = "Resolved navigation target did not match the fixture target."
        }
        if ($passed -and $case.execute) {
            $scopeFailures = @(Test-CommandScope $command $case.allowed_command_regex)
            if ($scopeFailures.Count -gt 0) {
                $passed = $false
                $failure = $scopeFailures -join " | "
            } else {
                $execution = Invoke-BoundedCommand $command
                if ($execution.timed_out) {
                    $passed = $false
                    $failure = $execution.stderr
                } elseif ($case.expected_exit_codes -notcontains [int]$execution.exit_code) {
                    $passed = $false
                    $failure = "Command exited with $($execution.exit_code); expected $($case.expected_exit_codes -join ', ')."
                }
                $postconditionFailures = @(Test-Postconditions $case $execution)
                if ($postconditionFailures.Count -gt 0) {
                    $passed = $false
                    $failure = $postconditionFailures -join " | "
                }
            }
        }
        if (-not $passed -and [string]::IsNullOrWhiteSpace($failure)) {
            $failure = "Expected action $($case.expected_actions -join ' or '), received $action."
        }
        $results.Add([pscustomobject]@{
            category = $case.category
            prompt = $case.prompt
            expected_actions = @($case.expected_actions)
            action = $action
            command = Redact-Text $command
            target = if ($null -ne $plan) { Redact-Text ([string]$plan.target) } else { $null }
            risk = if ($null -ne $plan) { $plan.risk } else { $null }
            needs_approval = if ($null -ne $plan) { $plan.needs_approval } else { $null }
            planner_reason = if ($null -ne $plan) { Redact-Text ([string]$plan.reason) } else { $null }
            fallback_message = if ($null -ne $plan) { Redact-Text ([string]$plan.fallback_message) } else { $null }
            parser_strategy = if ($null -ne $plan -and $null -ne $plan.diagnostics) { $plan.diagnostics.parser_strategy } else { $null }
            parse_errors = if ($null -ne $plan -and $null -ne $plan.diagnostics) { @($plan.diagnostics.parse_errors) } else { @() }
            approved_by_harness = [bool]($case.execute -and $action -eq "approval_required" -and $scopeFailures.Count -eq 0)
            executed = $null -ne $execution
            execution_exit_code = if ($null -ne $execution) { $execution.exit_code } else { $null }
            execution_stdout = if ($null -ne $execution) { Redact-Text $execution.stdout } else { $null }
            execution_stderr = if ($null -ne $execution) { Redact-Text $execution.stderr } else { $null }
            scope_failures = @($scopeFailures)
            postcondition_failures = @($postconditionFailures)
            process_exit_code = $processExit
            latency_ms = $watch.ElapsedMilliseconds
            passed = $passed
            failure_reason = Redact-Text $failure
        })
    }
} finally {
    Pop-Location
    if ($hadModelsDirectory) {
        $env:AISH_MODELS_DIR = $previousModelsDirectory
    } else {
        Remove-Item Env:AISH_MODELS_DIR -ErrorAction SilentlyContinue
    }
    Remove-DynamicFixture
}

$passedCount = @($results | Where-Object { $_.passed }).Count
$report = [pscustomobject]@{
    schema = "aish.dynamic-sandbox-acceptance.v1"
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    model_id = $Model
    fixture_path = "<fixture-root>"
    fixture_retained = [bool]$KeepFixture
    case_count = $results.Count
    passed_count = $passedCount
    failed_count = $results.Count - $passedCount
    executed_count = @($results | Where-Object { $_.executed }).Count
    safety_policy = "execute only scenario-matched commands whose absolute paths remain inside the generated fixture"
    results = @($results)
}
$outputDirectory = Split-Path -Parent $outputFullPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$serialized = $report | ConvertTo-Json -Depth 12
$serialized = Redact-Text $serialized
[System.IO.File]::WriteAllText(
    $outputFullPath,
    $serialized,
    [System.Text.UTF8Encoding]::new($false)
)

if ($KeepFixture) {
    Write-Host "Dynamic fixture retained: $fixtureRoot"
}
Write-Host "Dynamic sandbox acceptance: $passedCount/$($results.Count) passed."
Write-Host "Report: $outputFullPath"
