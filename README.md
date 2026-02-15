# dd-ferryman

Local DNS + HTTPS for `.test` domains on macOS.

dd-ferryman runs two servers — a **DNS server** on UDP and an **HTTPS server** on TCP — so that any `.test` domain resolves to your machine and gets a trusted TLS certificate automatically. The DNS server makes the domain work; the HTTPS server terminates TLS and proxies to your app.

## Installation

```bash
brew tap yizyace/dd-ferryman
brew install dd-ferryman
```

## Quick Start

```bash
# Start both servers (DNS on UDP 127.0.0.1:9253, HTTPS on TCP 0.0.0.0:443)
sudo dd-ferryman start

# Check if servers are running
sudo dd-ferryman status

# Stop servers
sudo dd-ferryman stop
```

## Install vs Start

dd-ferryman has two ways to run: **manual** (`start`/`stop`) and **installed** (`install`/`uninstall`).

| | Manual | Installed |
|---|--------|-----------|
| **Start** | `sudo dd-ferryman start` | `sudo dd-ferryman install` |
| **Stop** | `sudo dd-ferryman stop` | `sudo dd-ferryman uninstall` |
| **Survives reboot** | No | Yes |
| **Restarts on crash** | No | Yes |
| **Mechanism** | Spawns a background process | Creates a macOS launchd service |

**Manual mode** — `start` spawns a daemon process. `stop` sends it SIGTERM. If your machine reboots or the process crashes, dd-ferryman won't come back until you run `start` again.

**Installed mode** — `install` creates a LaunchDaemon plist at `/Library/LaunchDaemons/com.dd-ferryman.plist` and loads it via `launchctl`. macOS will start dd-ferryman at boot (`RunAtLoad`) and restart it if it dies (`KeepAlive`). `uninstall` removes the plist and stops the service.

Use `install` if you want dd-ferryman always available. Use `start`/`stop` for temporary sessions or debugging (pass `--foreground` to see logs in your terminal).

## What Happens on First Run

1. Generates a local CA certificate in `~/.dd-ferryman/ca/`
2. Trusts the CA in the macOS system keychain
3. Writes `/etc/resolver/test` so `.test` domains resolve to `127.0.0.1`

Subsequent runs reuse the existing CA and resolver config.

## How It Works

Three pieces work together to make `anything.test` resolve, get a trusted certificate, and reach your app. Two of these are servers that dd-ferryman runs simultaneously:

### 1. DNS server (UDP 127.0.0.1:9253) — redirect `.test` to localhost

macOS has a built-in mechanism for per-domain DNS: any file in `/etc/resolver/` named after a TLD overrides how that TLD is resolved. The file `/etc/resolver/test` tells macOS "for any `.test` domain, ask the DNS server at `127.0.0.1:9253` instead of the default resolver." dd-ferryman runs that DNS server and answers every query with `127.0.0.1`. The result: `myapp.test`, `api.test`, any `*.test` address all point to your machine — no `/etc/hosts` editing, no `dnsmasq`.

This is a UDP server because DNS uses UDP by default. It only listens on `127.0.0.1` (loopback) since it only needs to serve the local machine.

### 2. HTTPS server (TCP 0.0.0.0:443) — TLS + reverse proxy

This is a TCP server because HTTPS runs over TCP. It handles two jobs:

**TLS termination.** On first run, dd-ferryman creates a local Certificate Authority (CA) and adds it to your macOS keychain. When a browser connects to `myapp.test:443`, dd-ferryman generates a certificate for `myapp.test` on the fly, signed by that CA. Because your system already trusts the CA, the browser trusts the certificate — no warnings, no self-signed cert hacks.

**Reverse proxying.** The server reads the `Host` header from the incoming request (e.g. `myapp.test`), looks up `~/.dd-ferryman/apps/myapp` for a port number, and forwards the request to `http://127.0.0.1:<port>`. Your app just needs to listen on HTTP — dd-ferryman handles TLS termination.

### Why two servers?

DNS and HTTPS are fundamentally different protocols — DNS uses UDP datagrams while HTTPS uses TCP streams. They also serve different roles: the DNS server makes `.test` domains resolve to `127.0.0.1`, and the HTTPS server handles what happens when a browser connects to that address on port 443. Both must run for the full flow to work.

### Request flow

```
Browser requests https://myapp.test
        │
        ▼
macOS resolver ──▶ dd-ferryman DNS (127.0.0.1:9253)
                         │
                   resolves to 127.0.0.1
                         │
                         ▼
              dd-ferryman HTTPS (:443)
              ├─ generates TLS cert for myapp.test
              └─ proxies to http://127.0.0.1:<port>
                         │
                         ▼
                Your app (HTTP)
```

## Directory Layout

All state lives in `~/.dd-ferryman/`:

```
~/.dd-ferryman/
├── ca/
│   ├── ca.crt          # Local CA certificate (PEM)
│   └── ca.key          # CA private key (PEM, mode 0600)
├── apps/
│   └── <app-name>      # Plain text file containing a port number
├── ferryman.pid        # PID of the running daemon
└── ferryman.log        # Daemon stdout/stderr (append-only)
```

| Path | Purpose |
|------|---------|
| `ca/ca.crt` | Self-signed root CA certificate. Trusted in the macOS system keychain on first run. Used to sign per-domain leaf certificates at runtime. |
| `ca/ca.key` | CA private key. Restricted to owner-only read/write (`0600`). Used to sign leaf certificates on the fly. |
| `apps/<name>` | One file per app. The filename is the subdomain (e.g. `myapp` for `myapp.test`) and the contents are the port number your app listens on. Create these yourself — e.g. `echo 3000 > ~/.dd-ferryman/apps/myapp`. |
| `ferryman.pid` | Written on daemon start, removed on stop. Used to detect whether the daemon is already running. |
| `ferryman.log` | Captured stdout/stderr from the background daemon process. Appended to across restarts. |

The `ca/` and `apps/` directories are created automatically on first run. The `ferryman.pid` and `ferryman.log` files are created when the daemon starts.

## Why `sudo`?

`dd-ferryman start` requires root because three operations need it:

- **Port 443** — binding to ports below 1024 requires root on macOS
- **Keychain trust** — adding the local CA to the system keychain requires `security add-trusted-cert`, which needs root
- **`/etc/resolver/test`** — writing to `/etc/resolver/` requires root

After startup, the daemon continues running as root to keep the port 443 binding.

## Requirements

- macOS
