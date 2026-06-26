# Project Inspection

AiSH should inspect the project before asking Ken or any AI runtime to generate a command.

The model should not guess project type, package manager, framework, or available scripts from thin air.

## Inspection Flow

```text
cwd
  -> scan known project files
  -> detect project type
  -> detect package manager
  -> read scripts/tasks
  -> detect installed CLIs
  -> build compact context packet
  -> completion engine / Ken / planner
```

## Files To Detect

```text
JavaScript/TypeScript:
  package.json
  package-lock.json
  pnpm-lock.yaml
  yarn.lock
  bun.lockb
  vite.config.js
  vite.config.ts
  next.config.js
  next.config.mjs
  nuxt.config.ts
  angular.json
  svelte.config.js
  astro.config.mjs

Containers:
  Dockerfile
  docker-compose.yml
  docker-compose.yaml
  compose.yml
  compose.yaml

Flutter/Dart:
  pubspec.yaml

Cloud/deploy:
  firebase.json
  supabase/config.toml
  vercel.json
  netlify.toml
  wrangler.toml

Rust:
  Cargo.toml
  Cargo.lock

Python:
  pyproject.toml
  requirements.txt
  uv.lock
  poetry.lock
  Pipfile

.NET:
  *.csproj
  *.sln

Go:
  go.mod
  go.sum

Java/JVM:
  pom.xml
  build.gradle
  build.gradle.kts
  settings.gradle
  settings.gradle.kts

Git:
  .git
```

## Package Manager Detection

Priority for Node projects:

```text
pnpm-lock.yaml  -> pnpm
yarn.lock       -> yarn
bun.lockb       -> bun
package-lock.json -> npm
none            -> infer npm only if package.json exists and no lockfile exists
```

AiSH should prefer project evidence over generic defaults.

## Context Packet

Example:

```json
{
  "project_type": "vite-react",
  "package_manager": "pnpm",
  "node_modules_present": false,
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest"
  },
  "detected_files": [
    "package.json",
    "pnpm-lock.yaml",
    "vite.config.ts",
    "src/App.tsx"
  ],
  "available_tools": ["node", "pnpm", "git"]
}
```

Then Ken or the deterministic planner can correctly produce:

```text
pnpm install
pnpm run dev
```

## Recent Command Summaries

By default, AI prompts are single-shot.

AiSH should not automatically include old conversation context in every AI request.

Optional setting:

```text
include summaries of last 5 commands
```

Use compact summaries, not raw logs.

Example:

```json
{
  "recent_commands": [
    {
      "command": "npm run dev",
      "exit_code": 1,
      "summary": "Failed because port 5173 is already in use"
    }
  ]
}
```

## Available CLI Detection

AiSH should detect common tools with read-only checks.

Examples:

```text
node --version
npm --version
pnpm --version
yarn --version
git --version
docker --version
kubectl version --client
terraform version
python --version
cargo --version
```

These checks must be cached and rate-limited.

## Fixture-Based Eval Direction

Training and eval should include project-context fixtures.

Initial fixtures:

```text
vite_npm_no_node_modules
vite_pnpm_with_node_modules
next_vercel
firebase_hosting
supabase_local
flutter_app
docker_compose_project
cloudflare_wrangler
nestjs_yarn
python_fastapi
rust_cargo_project
```

Each fixture should test:

```text
- detected files
- package manager choice
- dependency-present vs dependency-missing plan
- command vs plan distinction
- build/test/lint/run distinction
- fallback when project evidence is missing
```

## Rule

```text
Inspect first. Generate second. Validate third. Execute only after safety.
```
