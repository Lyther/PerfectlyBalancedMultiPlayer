# NAT and Remote Access for SMMS

SMMS transport is HTTP on one host port (default `8730`, configurable via `--port` or config).

For remote play, the safest practical approach is a private VPN mesh (Tailscale/ZeroTier) and manifest signing.

## Recommended Setup (Tailscale)

1. Install [Tailscale](https://tailscale.com/download) on host and all clients.
2. Join the same tailnet.
3. Host runs `smms serve` (or `smms serve --port <port>`).
4. Host shares Tailscale IP with clients.
5. Clients run `smms fetch <tailscale-ip>` or `smms verify <tailscale-ip>`.

## Recommended Setup (ZeroTier)

1. Create and configure a [ZeroTier network](https://my.zerotier.com).
2. Join all machines to that network.
3. Authorize members in the controller.
4. Use the assigned private IP as host.

## Manifest Signing (Strongly Recommended)

Signing adds integrity/authenticity on untrusted networks.

1. On host: `smms gen-keypair`
2. Add generated `signing_key_path` to host `[host]` config.
3. Add generated `public_key` to each client under `[hosts.<host>]`.
4. Start host with `smms serve`.

If client has a configured host key and host serves unsigned manifest, sync is rejected.

## Port Forwarding (Fallback Only)

If VPN is impossible:

1. Forward host TCP port (default `8730`) from router to host machine.
2. Open host firewall for that port.
3. Clients use public IP and matching port.

## Security Warning

- Transport is not encrypted by default.
- Manifest signing protects integrity/authenticity, not confidentiality.
- Do not expose SMMS directly to the public internet unless you accept plaintext transfer risk.
