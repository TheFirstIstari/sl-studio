# Self-Hosted Runners

## Overview

SL Studio uses self-hosted runners for testing on Linux distributions not available as GitHub-hosted runners (Fedora, NixOS).

## Runner Configuration

### Labels

| Runner | Labels                                  |
| ------ | --------------------------------------- |
| Fedora | `self-hosted`, `fedora`, `linux`, `x64` |
| NixOS  | `self-hosted`, `nixos`, `linux`, `x64`  |

### Setup

1. Install GitHub Actions runner on the host machine
2. Configure with appropriate labels:

```bash
./config.sh --url https://github.com/TheFirstIstari/sl-studio \
  --token <TOKEN> \
  --labels self-hosted,fedora
```

3. Start the runner:

```bash
./run.sh
# Or as a service:
sudo ./svc.sh install
sudo ./svc.sh start
```

## CI Integration

Self-hosted runners run **Rust checks and tests only** (no Tauri builds):

- `rust-fedora`: Format check, clippy, build
- `tests-fedora`: `cargo test`
- `rust-nixos`: Format check, clippy, build
- `tests-nixos`: `cargo test`

### Mise Integration

Self-hosted runners use `mise` for task running:

```yaml
- name: Rust CI (mise)
  if: matrix.use-mise
  shell: bash
  run: |
    if command -v mise >/dev/null 2>&1; then
      mise run ci_rust
    else
      # Fallback
      (cd src-tauri && cargo fmt --check)
      (cd src-tauri && cargo clippy -- -D warnings)
      (cd src-tauri && cargo build --release)
    fi
```

## Troubleshooting

### Runner Not Picking Up Jobs

1. Verify runner is **Online** and **Idle** in GitHub Settings > Actions > Runners
2. Check labels match the job's `runs-on` specification
3. Ensure runner process is running:

```bash
ps -ef | grep actions-runner
# Or check service status:
systemctl status actions-runner
```

4. Check runner logs:

```bash
tail -f _diag/*.log
```

5. Verify network connectivity:

```bash
curl -I https://github.com
```

### Runner Busy

If the single runner is busy, jobs will queue. Consider:

- Adding a second runner for the same label
- Adjusting concurrency settings in the workflow
