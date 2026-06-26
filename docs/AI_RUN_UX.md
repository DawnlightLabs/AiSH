# AI Run UX

AI Run is the execution-focused AI submode.

In AI Run, the user types a natural-language terminal request. AiSH inspects context, asks Ken/runtime planner for a card, validates it, checks risk, and may execute it automatically only when the action is low-risk.

## AI Mode Submodes

```text
AI Suggest:
  Show generated command/plan/script before running.

AI Run:
  Run low-risk command/plan directly and show the result first.
```

AI Ask can exist as a side panel or command palette helper, but it is not the main execution submode.

## Execution Flow

```text
User intent
  -> AiSH inspects context
  -> AiSH retrieves CLI/tool docs if needed
  -> Ken/runtime planner predicts command/plan/script/fallback
  -> AiSH validates structure
  -> AiSH checks risk
  -> AiSH asks confirmation if needed
  -> AiSH executes if allowed
  -> AiSH shows result
  -> user can expand Command Trace / Working details
```

The model never directly executes anything.

## Low-Risk Auto-Run Candidates

Low-risk commands may auto-run in AI Run mode:

```text
- list files
- show git status
- show npm scripts
- show processes
- read logs
- search files
- inspect environment
- run tests
- show current branch
- check port owner
```

## Confirmation Required

Confirmation is required for:

```text
- install dependencies
- delete/move files
- git reset / git clean
- chmod / chown
- service changes
- registry edits
- cloud mutations
- deploy commands
- admin commands
- production commands
- terraform apply
- npm publish
```

## Result-First UI

AI Run should show the final result first.

Example request:

```text
find the 20 biggest folders in my user directory
```

Visible result:

```text
Top 20 largest folders
1. Downloads - 43.2 GB
2. AppData - 28.7 GB
...
```

The command details are hidden behind an expandable section.

## Trace Dropdown

Do not call this “thinking.” It should show what the app did, not hidden model reasoning.

Preferred UI labels:

```text
Primary compact label:
  Working

Technical/expanded label:
  Command Trace
```

Other acceptable labels:

```text
Run Details
Plan & Commands
```

The dropdown should show:

```text
- detected intent
- context used
- command card type
- commands run
- plan steps
- exit code
- duration
- logs
- command output
- safety decision
```

## Trace Example

```text
Working
  Intent: find largest folders under user profile
  Context: shell=powershell, cwd=C:\Users\Amaan
  Card: script
  Risk: low
  Command:
    PowerShell script for folder size scan
  Exit code: 0
  Duration: 8.3s
```

## Card Types

AI Run can execute only validated card types:

```text
Command Card:
  single command

Plan Card:
  multi-step workflow

Script Card:
  longer generated script for complex shell workflows

Fallback Message:
  no execution
```

Script Cards require extra scrutiny. Long scripts should usually be shown before execution unless they are read-only and clearly low-risk.

## Safety Defaults

```text
AI Suggest:
  never auto-run

AI Run:
  auto-run low-risk only
  confirm medium/high-risk
  never run fallback
  never run invalid cards
```
