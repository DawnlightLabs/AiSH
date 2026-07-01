# Docker-only testing

The host needs only Docker Desktop and Docker Compose. Rust, Node.js, Tauri,
Python, and AiSH are built and run inside containers.

## Desktop app

Build and start the native Linux Tauri app:

```bash
docker compose up --build -d desktop
```

Compose downloads and validates the shared GGUF model before starting the
desktop. To download or repair it explicitly:

```bash
docker compose run --rm model
```

Open:

```text
http://localhost:6080/vnc.html?autoconnect=true&resize=scale
```

The app runs inside a virtual Linux desktop. Test the embedded terminal with:

```bash
pwd
ls
git status --short
echo AISH_DOCKER_OK
```

The repository is mounted read-only at `/workspace`, so terminal commands cannot
modify host source files.

View logs or stop the desktop:

```bash
docker compose logs -f desktop
docker compose down
```

## Provider shell

Build and open the provider interactively:

```bash
docker compose build provider
docker compose run --rm provider
```

The image includes `llama-cli`. Compose downloads the GGUF model into the
persistent `aish-models` Docker volume before opening the provider. The download
is reused by both services.

Test direct shell behavior:

```text
/status
pwd
ls
git status --short
/help
/exit
```

Test approval without deleting anything:

```text
rm /tmp/does-not-exist
/cancel
```

The `rm` command must remain pending until `/approve`; `/cancel` discards it.
Natural-language requests work after the model download finishes.

## Cleanup

Stop containers without deleting Docker volumes:

```bash
docker compose down
```

Delete the test volumes as well:

```bash
docker compose down -v
```
