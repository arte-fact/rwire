---
title: Scaling & Deployment Model
description: What one rwire process gives you, where the ceilings are, and how to deploy behind a proxy
order: 7
---
# Scaling & Deployment Model

rwire is a **single-process, connection-oriented** server: every open tab is a
live WebSocket with its state in that process's memory. That model is a
deliberate fit for the framework's niche — self-hosted software, dashboards,
internal tools — and this page states plainly what it gives you, where the
ceilings are, and what to do at them.

## Memory: what a connection costs

A connection's footprint is dominated by *your application state*, not the
framework. The framework's own per-connection bookkeeping — symbol table,
sent-names/CSS dedup sets, synced-region hashes, socket buffers — is small
(a minimal app has held 5,000 live connections in under 4&nbsp;MB total). Budget
for: `your state size × open connections`, plus the shared/persisted cache
(one instance per state type, not per connection).

Guard rails, all on by default:

| Limit | Default | Where |
|---|---|---|
| Total connections | 10,000 | `ServerConfig::max_connections` |
| Connections per IP | 100 | `ServerConfig::max_connections_per_ip` |
| Idle disconnect | 5 min | `ServerConfig::idle_timeout` |
| State memory per connection | 1 MB | `ServerConfig::state_memory_limit` |
| Inbound events per connection | 100/s (100 burst) | token bucket, built in |
| Inbound WS message / total | 64 KB frame path, 256 KB message | built in |
| Disconnected-session cache | 10,000 sessions, 5 min TTL | built in |

Over-limit connections get a `503` before the upgrade; `/health` and `/ready`
answer even at capacity, so probes and load balancers see the truth.

## Reconnects and deploys

The client reconnects with exponential backoff (cap 30s) and re-requests its
route; **memory state does not survive a disconnect beyond the 5-minute
session cache**, while `persisted` state survives anything (it's in SQLite).
After a deploy, reconnecting clients probe `/ready` and reload themselves —
they always run the new capsule, so there is no protocol version skew, ever.

## Horizontal scaling: partition, don't pool

There is deliberately **no multi-process story** in 0.x. The two mechanisms
that look like they might cross processes don't:

- `#[storage(shared)]` broadcasts over **in-process** channels. Two processes
  are two separate rooms.
- `#[storage(persisted)]` writes a **single SQLite file** (WAL mode). Run
  exactly one process per database file.

So scale by **partitioning**, not pooling: one process per app, per team, per
tenant — each with its own port and database. If you must run several
replicas of one app, the load balancer needs **sticky sessions** (the
`rwire_sid` cookie or IP hash), users on different replicas won't share
`shared` state, and each replica needs its own SQLite file. At that point,
honestly evaluate whether the app has outgrown the framework's niche.

## Behind a reverse proxy

The intended production shape: TLS terminates at a proxy (nginx, Caddy,
Traefik) that forwards WebSocket upgrades and sets `X-Forwarded-Proto: https`
(session cookies then auto-mark `Secure`). Same-origin deployments need no
Origin configuration; a page on another domain needs
`ServerConfig::allow_origin`. To mount an app under a path prefix, give the
capsule a `base_path` and strip the prefix at the proxy — the server stays
path-unaware. Prometheus metrics are at `/metrics` for capacity tracking
(active connections, rejects, event rates).
