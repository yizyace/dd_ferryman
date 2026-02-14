# dd-ferryman

Local DNS + HTTPS for `.test` domains on macOS.

## Quick Start

```bash
# Start servers (DNS on 127.0.0.1:9253, HTTPS on 0.0.0.0:443)
sudo dd-ferryman start

# Check if servers are running
sudo dd-ferryman status

# Stop servers
sudo dd-ferryman stop
```

## What Happens on First Run

1. Generates a local CA certificate in `~/.dd-ferryman/ca/`
2. Trusts the CA in the macOS system keychain
3. Writes `/etc/resolver/test` so `.test` domains resolve to `127.0.0.1`

Subsequent runs reuse the existing CA and resolver config.

## How It Works

- A DNS server on `127.0.0.1:9253` resolves all `.test` queries to `127.0.0.1`
- An HTTPS server on `0.0.0.0:443` generates per-domain TLS certificates on the fly, signed by the local CA
- macOS resolver integration means `anything.test` just works in browsers and CLI tools

## Requirements

- macOS
- `sudo` (needed for port 443, keychain trust, and `/etc/resolver/test`)
