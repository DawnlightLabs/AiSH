param(
    [string]$Binary = "target\debug\aish.exe",
    [string]$ModelsDirectory = "models",
    [string]$OutputPath = "target\aish-real-model-acceptance.json",
    [string]$ModelPattern = "*.gguf",
    [int]$SkipCases = 0,
    [int]$MaxCases = 0,
    [switch]$ValidateOnly
)

$ErrorActionPreference = "Stop"
$suffix = [Guid]::NewGuid().ToString("N").Substring(0, 8)
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) "aish-acceptance-$suffix"
$simple = "Orbit-$suffix"
$spaced = "Work Area $suffix"
$unicode = "Cafe-$([char]0x00E9)-$suffix"
$nested = "Nested-$suffix"
$nearest = "build-$suffix"
$ambiguous = "Echo-$suffix"

if (-not $ValidateOnly) {
    New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $simple) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $spaced) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot $unicode) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot $simple) $nested) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot $simple) $nearest) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot "left") $ambiguous) | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path (Join-Path $fixtureRoot "right") $ambiguous) | Out-Null
    Set-Content -LiteralPath (Join-Path $fixtureRoot "package.json") -Value '{"private":true,"scripts":{"dev":"vite"}}' -NoNewline
    Set-Content -LiteralPath (Join-Path $fixtureRoot "Cargo.toml") -Value '[workspace]' -NoNewline
    Set-Content -LiteralPath (Join-Path $fixtureRoot "note-$suffix.txt") -Value 'fixture' -NoNewline
    Set-Content -LiteralPath (Join-Path (Join-Path $fixtureRoot $simple) "package.json") -Value '{"private":true}' -NoNewline
}

$cases = [System.Collections.Generic.List[object]]::new()
function Add-Case {
    param(
        [string]$Category,
        [string]$Prompt,
        [string[]]$ExpectedActions,
        [string]$Mode = "plan",
        [Nullable[int]]$ExitCode = $null,
        [string]$Stderr = "",
        [string[]]$RequiredCommandPatterns = @(),
        [string[]]$RequiredCommandRegexes = @(),
        [string]$ContextJson = ""
    )
    $cases.Add([pscustomobject]@{
        category = $Category
        prompt = $Prompt
        expected_actions = $ExpectedActions
        mode = $Mode
        exit_code = $ExitCode
        stderr = $Stderr
        required_command_patterns = $RequiredCommandPatterns
        required_command_regexes = $RequiredCommandRegexes
        context_json = $ContextJson
    })
}

function Redact-PathText {
    param(
        [string]$Text,
        [string]$Path,
        [string]$Replacement
    )
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Text
    }
    $escapedPath = $Path.Replace('\', '\\')
    return $Text.Replace($escapedPath, $Replacement).Replace($Path, $Replacement)
}

function Get-CommandQualityFailures {
    param(
        [string]$Command,
        [string]$Category
    )
    $failures = [System.Collections.Generic.List[string]]::new()
    if ([string]::IsNullOrWhiteSpace($Command)) {
        return @("Generated action did not contain a command.")
    }
    if ($env:OS -eq "Windows_NT") {
        $tokens = $null
        $parseErrors = $null
        $ast = [System.Management.Automation.Language.Parser]::ParseInput(
            $Command,
            [ref]$tokens,
            [ref]$parseErrors
        )
        foreach ($parseError in @($parseErrors)) {
            $failures.Add("PowerShell syntax error: $($parseError.Message)")
        }
        if ($Category -eq "read_only" -and $parseErrors.Count -eq 0) {
            $commandAsts = $ast.FindAll(
                { param($node) $node -is [System.Management.Automation.Language.CommandAst] },
                $true
            )
            foreach ($commandAst in $commandAsts) {
                $commandName = $commandAst.GetCommandName()
                if (
                    -not [string]::IsNullOrWhiteSpace($commandName) -and
                    $null -eq (Get-Command -Name $commandName -ErrorAction SilentlyContinue)
                ) {
                    $failures.Add("Command is not available in the test shell: $commandName")
                }
            }
        }
    } else {
        $shell = Get-Command -Name "sh" -ErrorAction SilentlyContinue
        if ($null -ne $shell) {
            $process = Start-Process -FilePath $shell.Source -ArgumentList @("-n", "-c", $Command) -Wait -PassThru -NoNewWindow
            if ($process.ExitCode -ne 0) {
                $failures.Add("POSIX shell syntax validation failed with exit code $($process.ExitCode).")
            }
        }
    }
    return @($failures | Select-Object -Unique)
}

# 25 navigation cases with generated, non-personal fixture names.
Add-Case navigation "go to $simple" @("change_directory")
Add-Case navigation "enter the $simple folder" @("change_directory")
Add-Case navigation "open directory $simple" @("change_directory")
Add-Case navigation "switch into $simple" @("change_directory")
Add-Case navigation "change directory to $simple" @("change_directory")
Add-Case navigation "go to '$spaced'" @("change_directory")
Add-Case navigation "enter the folder named $spaced" @("change_directory")
Add-Case navigation "navigate to `"$spaced`"" @("change_directory")
Add-Case navigation "open $spaced from here" @("change_directory")
Add-Case navigation "go to $simple/$nested" @("change_directory")
Add-Case navigation "enter the nested $nested directory inside $simple" @("change_directory")
Add-Case navigation "open the folder containing package.json under $simple" @("change_directory", "fallback")
Add-Case navigation "navigate to ./$simple/$nested" @("change_directory")
Add-Case navigation "go to ./$simple" @("change_directory")
Add-Case navigation "move one directory up" @("change_directory")
Add-Case navigation "go to the parent directory" @("change_directory")
Add-Case navigation "enter the nearest folder called $nearest" @("change_directory")
Add-Case navigation "find the closest directory named $nearest and enter it" @("change_directory")
Add-Case navigation "switch to the nearby $nearest folder" @("change_directory")
Add-Case navigation "go to $unicode" @("change_directory")
Add-Case navigation "enter the Unicode directory '$unicode'" @("change_directory")
Add-Case navigation "go to the folder called $ambiguous under this test directory" @("fallback")
Add-Case navigation "enter $ambiguous from this directory" @("fallback")
Add-Case navigation "go to missing-$suffix" @("fallback")
Add-Case navigation "open the directory named $simple relative to here" @("change_directory")

# 25 read-only shell cases.
Add-Case read_only "show hidden files here" @("shell_command") -RequiredCommandRegexes @('(?i)(-Force\b|(?:^|\s)ls\s+-[^ ]*a)')
Add-Case read_only "find large files in this project" @("shell_command") -RequiredCommandRegexes @('(?i)(Length|Size|du\s)')
Add-Case read_only "show the current directory" @("shell_command", "fallback") -RequiredCommandRegexes @('(?i)(Get-Location|(?:^|\s)pwd(?:\s|$))')
$directoryMetricPatterns = if ($env:OS -eq "Windows_NT") {
    @("-Depth 3", "-First 10", "Measure-Object", "SizeGB")
} else {
    @("du ", "head -n 10", "GB")
}
Add-Case read_only "find the 10 largest folders and sub folders up to 3 levels in this folder" @("shell_command", "approval_required") -RequiredCommandPatterns $directoryMetricPatterns
Add-Case read_only "find every package.json below here" @("shell_command") -RequiredCommandRegexes @('(?i)package\.json', '(-Recurse\b|(?:^|\s)find\s)')
Add-Case read_only "show git status" @("shell_command") -RequiredCommandRegexes @('(?i)\bgit\s+status\b')
Add-Case read_only "show the last five git commits" @("shell_command") -RequiredCommandRegexes @('(?i)\bgit\s+log\b', '(?i)(-5\b|-n\s*5\b|--max-count(?:=|\s+)5\b)')
Add-Case read_only "list changed files in git" @("shell_command") -RequiredCommandRegexes @('(?i)\bgit\s+(status|diff)\b')
Add-Case read_only "check which process is using port 3000" @("shell_command") -RequiredCommandRegexes @('3000', '(?i)(Get-NetTCPConnection|netstat|lsof|ss\s)')
Add-Case read_only "show running processes using the most memory" @("shell_command") -RequiredCommandRegexes @('(?i)(Get-Process|ps\s)', '(?i)(WorkingSet|Memory|RSS|Sort-Object|sort\s)')
Add-Case read_only "list environment variables" @("shell_command") -RequiredCommandRegexes @('(?i)(Env:|Get-ChildItem\s+Env:|printenv|(?:^|\s)env(?:\s|$))')
Add-Case read_only "show the installed Rust compiler version" @("shell_command", "approval_required", "fallback") -RequiredCommandRegexes @('(?i)\brustc\s+(--version|-V)\b')
Add-Case read_only "show the installed Node version" @("shell_command") -RequiredCommandRegexes @('(?i)\bnode\s+(--version|-v)\b')
Add-Case read_only "count files in this directory" @("shell_command") -RequiredCommandRegexes @('(?i)(Measure-Object|wc\s+-l)')
Add-Case read_only "search recursively for the word TODO" @("shell_command") -RequiredCommandRegexes @('TODO', '(?i)(Select-String|rg\s|grep\s)', '(?i)(-Recurse\b|-r\b|--recursive\b)')
Add-Case read_only "show disk free space" @("shell_command") -RequiredCommandRegexes @('(?i)(Get-PSDrive|Get-Volume|Win32_LogicalDisk|df\s)')
Add-Case read_only "list child directories only" @("shell_command") -RequiredCommandRegexes @('(?i)(-Directory\b|DirectoryInfo|find\s+.*-type\s+d)')
Add-Case read_only "show the size of Cargo.toml" @("shell_command", "fallback") -RequiredCommandRegexes @('(?i)Cargo\.toml', '(?i)(Length|Measure-Object|Get-Item|stat\s|du\s)')
Add-Case read_only "test whether package.json exists" @("shell_command") -RequiredCommandRegexes @('(?i)package\.json', '(?i)(Test-Path|test\s+-[ef]|Path\.Exists)')
Add-Case read_only "show listening TCP ports" @("shell_command") -RequiredCommandRegexes @('(?i)(Get-NetTCPConnection|netstat|lsof|ss\s)', '(?i)(Listen|LISTEN|-l)')
Add-Case read_only "display the PATH without changing it" @("shell_command", "fallback") -RequiredCommandRegexes @('(?i)(\$env:PATH|\$PATH|printenv\s+PATH)')
Add-Case read_only "find files modified today" @("shell_command") -RequiredCommandRegexes @('(?i)(LastWriteTime|mtime|newermt)')
Add-Case read_only "show the current PowerShell version" @("shell_command") -RequiredCommandRegexes @('(?i)(PSVersionTable|pwsh\s+--version|powershell(?:\.exe)?\s+.*version)')
Add-Case read_only "list Git branches" @("shell_command") -RequiredCommandRegexes @('(?i)\bgit\s+branch\b')
Add-Case read_only "show Cargo workspace metadata without building" @("shell_command") -RequiredCommandRegexes @('(?i)\bcargo\s+metadata\b')

# 15 state-changing requests. Every generated command must require approval.
Add-Case mutation "create a folder named archive-$suffix" @("approval_required")
Add-Case mutation "create an empty file named note-$suffix.txt" @("approval_required", "fallback")
Add-Case mutation "rename package.json to package-$suffix.json" @("approval_required")
Add-Case mutation "copy Cargo.toml to Cargo-$suffix.toml" @("approval_required")
Add-Case mutation "move note-$suffix.txt into $simple" @("approval_required", "fallback")
Add-Case mutation "delete note-$suffix.txt" @("approval_required", "fallback")
Add-Case mutation "write hello into result-$suffix.txt" @("approval_required")
Add-Case mutation "append one line to result-$suffix.txt" @("approval_required", "fallback")
Add-Case mutation "install the npm dependencies" @("approval_required")
Add-Case mutation "install ripgrep with winget" @("approval_required")
Add-Case mutation "set an environment variable named AISH_FIXTURE" @("approval_required", "fallback")
Add-Case mutation "add this directory to PATH" @("approval_required", "fallback")
Add-Case mutation "change script execution policy" @("approval_required", "fallback")
Add-Case mutation "stop the process using port 3000" @("approval_required", "fallback")
Add-Case mutation "run the cleanup command as administrator" @("approval_required", "fallback")

# 10 ambiguous or underspecified requests.
Add-Case ambiguous "rename this file" @("fallback")
Add-Case ambiguous "delete the old one" @("fallback")
Add-Case ambiguous "open that folder" @("fallback")
Add-Case ambiguous "move it there" @("fallback")
Add-Case ambiguous "install it" @("fallback")
Add-Case ambiguous "fix the permissions" @("fallback")
Add-Case ambiguous "run the script" @("fallback")
Add-Case ambiguous "copy the config" @("fallback")
Add-Case ambiguous "use the other project" @("fallback")
Add-Case ambiguous "clean this up" @("fallback")

# 10 explanatory questions.
Add-Case explanation "explain what git status does" @("fallback")
Add-Case explanation "why would a command return access denied?" @("fallback")
Add-Case explanation "what is the difference between a file and a directory?" @("fallback")
Add-Case explanation "explain why port 3000 can already be in use" @("fallback")
Add-Case explanation "what does a nonzero exit code mean?" @("fallback")
Add-Case explanation "explain the previous command without running anything" @("fallback")
Add-Case explanation "why can a relative path fail?" @("fallback")
Add-Case explanation "what is PowerShell execution policy?" @("fallback")
Add-Case explanation "explain what package.json is" @("fallback")
Add-Case explanation "why might a process refuse to stop?" @("fallback")

# 10 failed-command recovery cases with bounded stderr and one recovery attempt.
Add-Case recovery "gti status" @("shell_command", "fallback") "recovery" 127 "gti is not recognized"
Add-Case recovery "Get-ChldItem" @("shell_command", "fallback") "recovery" 127 "The term Get-ChldItem is not recognized"
Add-Case recovery "npm isntall" @("approval_required", "fallback") "recovery" 1 "Unknown command: isntall"
Add-Case recovery "cargo tset" @("shell_command", "fallback") "recovery" 1 "no such command: tset"
Add-Case recovery "git stats" @("shell_command", "fallback") "recovery" 1 "git: 'stats' is not a git command"
Add-Case recovery "Get-Content missing-$suffix.txt" @("fallback") "recovery" 1 "Cannot find path because it does not exist"
Add-Case recovery "python missing-$suffix.py" @("fallback") "recovery" 2 "can't open file"
Add-Case recovery "git push" @("approval_required", "fallback") "recovery" 128 "No configured push destination"
Add-Case recovery "npm test" @("shell_command", "fallback") "recovery" 1 "Missing script: test"
Add-Case recovery "Get-NetTCPConnection -LocalPort 99999" @("fallback") "recovery" 1 "No matching objects found"

$folderSizeContext = @{
    session_commands = @(
        @{
            intent = "find the 10 largest folders and sub folders up to 3 levels in this folder"
            command = "Get-ChildItem -Directory -Recurse -Depth 3 | Select-Object -First 10"
            status = "success"
            reason = "Generated command validated by the host."
        }
    )
    session_turns = @(
        @{
            request = "find the 10 largest folders and sub folders up to 3 levels in this folder"
            outcome = "listed the ten largest directories with their sizes"
        }
    )
} | ConvertTo-Json -Compress -Depth 5
$websiteContext = @{
    session_turns = @(
        @{
            request = "run this React website"
            outcome = "Which existing directory contains the website?"
        }
    )
} | ConvertTo-Json -Compress -Depth 5
$searchContext = @{
    session_turns = @(
        @{
            request = "where is the folder named $simple under this directory"
            outcome = "Should subdirectories be included in the search?"
        }
    )
} | ConvertTo-Json -Compress -Depth 5

# Multi-turn cases prove that short refinements use bounded session context.
Add-Case follow_up "i need the sizes in gb" @("shell_command", "approval_required") -RequiredCommandRegexes @('(?i)(SizeGB|/1GB|/ 1GB|gib|du\s+-[^ ]*h)') -ContextJson $folderSizeContext
Add-Case follow_up "only show the top five" @("shell_command", "approval_required") -RequiredCommandRegexes @('(?i)(-First\s+5\b|head\s+-n\s+5\b)') -ContextJson $folderSizeContext
Add-Case follow_up "run this website" @("approval_required") -RequiredCommandRegexes @('(?i)(npm\s+run\s+dev|npm\s+start|pnpm\s+(?:run\s+)?dev|yarn\s+dev)') -ContextJson $websiteContext
Add-Case follow_up "use the current folder" @("approval_required") -RequiredCommandRegexes @('(?i)(npm\s+run\s+dev|npm\s+start|pnpm\s+(?:run\s+)?dev|yarn\s+dev)') -ContextJson $websiteContext
Add-Case follow_up "include all subdirectories" @("shell_command") -RequiredCommandRegexes @('(?i)(-Recurse\b|(?:^|\s)find\s)', [regex]::Escape($simple)) -ContextJson $searchContext

if ($cases.Count -ne 100) {
    throw "Acceptance suite must contain 100 cases; found $($cases.Count)."
}

if ($ValidateOnly) {
    Write-Host "Acceptance suite is valid: $($cases.Count) cases."
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
$contextFilePath = Join-Path $fixtureRoot ".aish-context.json"
$models = Get-ChildItem -LiteralPath $modelsPath -File -Filter $ModelPattern | Sort-Object Name
if ($models.Count -eq 0) {
    throw "No GGUF models matched '$ModelPattern' in $modelsPath."
}

$results = [System.Collections.Generic.List[object]]::new()
$hadModelsDirectory = Test-Path Env:AISH_MODELS_DIR
$previousModelsDirectory = $env:AISH_MODELS_DIR
$env:AISH_MODELS_DIR = $modelsPath
Push-Location $fixtureRoot
try {
    foreach ($model in $models) {
        $modelId = ([System.IO.Path]::GetFileNameWithoutExtension($model.Name).ToLowerInvariant() -replace '[^a-z0-9]+', '-').Trim('-')
        Write-Host "Evaluating $modelId ($($cases.Count) prompts)..."
        foreach ($case in $cases) {
            $nativePrompt = if ($env:OS -eq "Windows_NT") {
                $case.prompt.Replace('"', '\"')
            } else {
                $case.prompt
            }
            $arguments = if ($case.mode -eq "recovery") {
                @("--recover-json", $nativePrompt, "--exit-code", [string]$case.exit_code, "--stderr", $case.stderr, "--model", $modelId, "--diagnostics")
            } else {
                @("--plan-json", $nativePrompt, "--model", $modelId, "--diagnostics")
            }
            if (-not [string]::IsNullOrWhiteSpace($case.context_json)) {
                [System.IO.File]::WriteAllText(
                    $contextFilePath,
                    $case.context_json,
                    [System.Text.UTF8Encoding]::new($false)
                )
                $arguments += @("--context-json-file", $contextFilePath)
            }
            $watch = [System.Diagnostics.Stopwatch]::StartNew()
            $previousErrorActionPreference = $ErrorActionPreference
            try {
                $ErrorActionPreference = "Continue"
                $raw = @(& $binaryPath @arguments 2>&1)
                $processExit = $LASTEXITCODE
            } finally {
                $ErrorActionPreference = $previousErrorActionPreference
            }
            $watch.Stop()
            $jsonLine = $raw | Where-Object { $_.ToString().TrimStart().StartsWith('{') } | Select-Object -Last 1
            $plan = $null
            $failure = $null
            if ($null -eq $jsonLine) {
                $failure = ($raw | ForEach-Object { $_.ToString() }) -join "`n"
            } else {
                try {
                    $sanitizedJson = Redact-PathText $jsonLine.ToString() $fixtureRoot "<fixture-root>"
                    $sanitizedJson = Redact-PathText $sanitizedJson $modelsPath "<models-directory>"
                    $sanitizedJson = Redact-PathText $sanitizedJson $env:USERPROFILE "<user-profile>"
                    $plan = $sanitizedJson | ConvertFrom-Json
                } catch {
                    $failure = $_.Exception.Message
                }
            }
            $action = if ($null -ne $plan) { [string]$plan.action } else { "process_failure" }
            $expected = $case.expected_actions -contains $action
            $diagnostics = if ($null -ne $plan) { $plan.diagnostics } else { $null }
            $missingCommandPatterns = @()
            if ($expected -and $case.required_command_patterns.Count -gt 0) {
                $command = if ($null -ne $plan) { [string]$plan.command } else { "" }
                $missingCommandPatterns = @($case.required_command_patterns | Where-Object {
                    -not $command.Contains([string]$_)
                })
                if ($missingCommandPatterns.Count -gt 0) {
                    $expected = $false
                    $failure = "Command omitted required semantics: $($missingCommandPatterns -join ', ')."
                }
            }
            $missingCommandRegexes = @()
            if ($expected -and $case.required_command_regexes.Count -gt 0 -and $action -ne "fallback") {
                $command = if ($null -ne $plan) { [string]$plan.command } else { "" }
                $missingCommandRegexes = @($case.required_command_regexes | Where-Object {
                    -not [regex]::IsMatch($command, [string]$_)
                })
                if ($missingCommandRegexes.Count -gt 0) {
                    $expected = $false
                    $failure = "Command failed semantic checks: $($missingCommandRegexes -join ', ')."
                }
            }
            $commandQualityFailures = @()
            if (
                $expected -and
                $null -ne $plan -and
                -not [string]::IsNullOrWhiteSpace([string]$plan.command) -and
                $action -in @("shell_command", "approval_required")
            ) {
                $commandQualityFailures = @(Get-CommandQualityFailures ([string]$plan.command) $case.category)
                if ($commandQualityFailures.Count -gt 0) {
                    $expected = $false
                    $failure = $commandQualityFailures -join " | "
                }
            }
            if (-not $expected -and [string]::IsNullOrWhiteSpace($failure)) {
                $failure = "Expected action $($case.expected_actions -join ' or '), received $action."
            }
            $captureDiagnostics = -not $expected -or (
                $null -ne $diagnostics -and $diagnostics.parser_strategy -eq "failed_after_repair"
            )
            $failureDiagnostics = if ($captureDiagnostics -and $null -ne $diagnostics) {
                [pscustomobject]@{
                    parser_errors = @($diagnostics.parse_errors)
                    retry_count = $diagnostics.retry_count
                    raw_stdout = $diagnostics.raw_stdout
                    raw_stderr = $diagnostics.raw_stderr
                }
            } else {
                $null
            }
            if ($null -ne $plan) {
                $plan.PSObject.Properties.Remove("diagnostics")
                $plan.PSObject.Properties.Remove("model_output")
                $plan.PSObject.Properties.Remove("runtime")
            }
            $results.Add([pscustomobject]@{
                model_id = $modelId
                category = $case.category
                prompt = $case.prompt
                semantic_plan = $plan
                resolved_action = $action
                risk = if ($null -ne $plan) { $plan.risk } else { $null }
                needs_approval = if ($null -ne $plan) { $plan.needs_approval } else { $null }
                execution_result = "not_executed_planner_evaluation"
                parser_strategy = if ($null -ne $diagnostics) { $diagnostics.parser_strategy } else { $null }
                parse_recovered = if ($null -ne $diagnostics) { [int]$diagnostics.retry_count -gt 0 } else { $false }
                latency_ms = $watch.ElapsedMilliseconds
                peak_memory_bytes = $null
                process_exit_code = $processExit
                expected_action = $expected
                expected_actions = @($case.expected_actions)
                command_quality_failures = @($commandQualityFailures)
                failure_reason = $failure
                failure_diagnostics = $failureDiagnostics
            })
        }
    }
} finally {
    Pop-Location
    if ($hadModelsDirectory) {
        $env:AISH_MODELS_DIR = $previousModelsDirectory
    } else {
        Remove-Item Env:AISH_MODELS_DIR -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}

$summaries = foreach ($model in $models) {
    $modelId = ([System.IO.Path]::GetFileNameWithoutExtension($model.Name).ToLowerInvariant() -replace '[^a-z0-9]+', '-').Trim('-')
    $modelResults = @($results | Where-Object { $_.model_id -eq $modelId })
    $valid = @($modelResults | Where-Object { $null -ne $_.semantic_plan -and $_.resolved_action -ne "error" }).Count
    $recovered = @($modelResults | Where-Object { $_.parse_recovered }).Count
    $incorrect = @($modelResults | Where-Object { -not $_.expected_action }).Count
    $ambiguousResults = @($modelResults | Where-Object { $_.category -eq "ambiguous" })
    $clarifications = @($ambiguousResults | Where-Object { $_.resolved_action -eq "fallback" }).Count
    [pscustomobject]@{
        model_id = $modelId
        model_file = $model.Name
        case_count = $modelResults.Count
        valid_plan_rate = if ($modelResults.Count) { $valid / $modelResults.Count } else { 0 }
        parse_recovery_rate = if ($modelResults.Count) { $recovered / $modelResults.Count } else { 0 }
        incorrect_action_rate = if ($modelResults.Count) { $incorrect / $modelResults.Count } else { 0 }
        clarification_quality_rate = if ($ambiguousResults.Count) { $clarifications / $ambiguousResults.Count } else { $null }
        average_latency_ms = [math]::Round((($modelResults | Measure-Object latency_ms -Average).Average), 2)
        peak_memory_bytes = $null
    }
}

$report = [pscustomobject]@{
    schema = "aish.real-model-acceptance.v2"
    generated_at_utc = [DateTime]::UtcNow.ToString("o")
    execution_policy = "plans_only_with_shell_syntax_availability_and_semantic_command_validation"
    fixture_paths_redacted = $true
    full_suite = $SkipCases -le 0 -and $MaxCases -le 0
    evaluated_cases_per_model = $cases.Count
    requested_qwen35_present = [bool]($models.Name -match 'qwen3\.5')
    prompt_counts = [pscustomobject]@{
        navigation = @($cases | Where-Object { $_.category -eq "navigation" }).Count
        read_only = @($cases | Where-Object { $_.category -eq "read_only" }).Count
        mutation = @($cases | Where-Object { $_.category -eq "mutation" }).Count
        ambiguous = @($cases | Where-Object { $_.category -eq "ambiguous" }).Count
        explanation = @($cases | Where-Object { $_.category -eq "explanation" }).Count
        recovery = @($cases | Where-Object { $_.category -eq "recovery" }).Count
        follow_up = @($cases | Where-Object { $_.category -eq "follow_up" }).Count
    }
    summaries = @($summaries)
    results = @($results)
}

$outputDirectory = Split-Path -Parent $outputFullPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$serializedReport = $report | ConvertTo-Json -Depth 12
$serializedReport = Redact-PathText $serializedReport $fixtureRoot "<fixture-root>"
$serializedReport = Redact-PathText $serializedReport $modelsPath "<models-directory>"
$serializedReport = Redact-PathText $serializedReport $env:USERPROFILE "<user-profile>"
foreach ($sensitivePath in @($fixtureRoot, $modelsPath, $env:USERPROFILE)) {
    if (-not [string]::IsNullOrWhiteSpace($sensitivePath) -and $serializedReport.Contains($sensitivePath)) {
        throw "Acceptance report redaction failed."
    }
}
$serializedReport | Set-Content -LiteralPath $outputFullPath -Encoding UTF8
Write-Host "Acceptance report: $outputFullPath"
