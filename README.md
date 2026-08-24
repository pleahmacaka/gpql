# GPQL

A local-first SQL client. Connections, queries and settings live on your machine.
Read only is the default: GPQL asks the server to refuse writes before it runs a
statement.

```
bun install
bun run app     # desktop app (Tauri)
bun run web     # marketing site, account and sync API
```

Needs Rust, bun, and the WebView2 runtime on Windows.

## What it does

- **Data** — virtualized grid (rows and columns), resizable columns, per-column filter, bar/line charts
- **Query** — tree-sitter highlighting, run only the text you selected, saved queries, completion from the schema or a language server
- **Schema** — foreign keys drawn as a graph, arrow-key navigation, auto layout, `@gpql:comment` annotations
- **Quick connect** — scans loopback ports, and Tailscale peers, for databases you can already reach
- **Ctrl+K** — jump to a table, a saved query or a setting

## Databases

| Database | How it connects | Notes |
| --- | --- | --- |
| PostgreSQL | native wire protocol, rustls | TLS is tried first, `prefer` by default |
| MySQL | native wire protocol | |
| SQLite | local file | |
| Supabase | Postgres wire, TLS required | project ref expands to the db host |
| GreptimeDB | Postgres wire, port 4003 | |
| ClickHouse | HTTP, `JSONCompact` | |
| Turso | libSQL HTTP pipeline | |
| Cloudflare D1 | HTTP API | account id, database id, API token |
| InfluxDB 3 | HTTP, SQL | v1 and v2 are not supported |
| Snowflake | SQL REST API v2 | programmatic access token |
| Neo4j | Query API v2, Cypher | |
| FalkorDB | Redis protocol, Cypher | |
| DuckDB | bundled engine, local file | |

Not supported: FoundationDB. Its Rust binding needs the FoundationDB client
library present at build time, so linking it would break the build for everyone
who does not already have that library installed.

## Security, stated plainly

Please read this before pointing GPQL at anything that matters.

**Transport.** Postgres connections try TLS (rustls, webpki roots) and fall back
to plaintext when the server refuses, which is the `prefer` behaviour libpq also
has. Choose `require` per connection if you never want the fallback. MySQL uses
its driver's TLS support. The HTTP backends are HTTPS whenever the URL you give
them is. Certificate pinning is not implemented.

**Credentials at rest.** On Windows, saved logins and API keys are sealed with
DPAPI (`CryptProtectData`) for the current Windows account and stored in
`~/.gpql-logins`. On macOS and Linux that file is currently **plaintext JSON** —
the OS keyring is not wired up yet. If you are not on Windows, treat that file as
a secret you are storing in the clear.

**The sync token is a session token.** Signing in hands the desktop app your
better-auth session token, and the app sends it as a bearer token to the sync
API. It is not a scoped API key, so anything that reads it can act as your
account on this service until it expires. Scoped keys are the intended fix.

**What sync carries.** Preferences, the list of connections you have opened
(host, port, user, database name) and saved queries. Passwords are never sent.

**AI.** Providers are bring-your-own-key and calls go straight from your machine
to the endpoint you configured. GPQL does not proxy prompts. An ACP agent, if you
configure one, runs as a local child process.

## Layout

```
apps/desktop     Tauri app, Rust backend
apps/web         SvelteKit site, better-auth, sync API
packages/ui      the components both of them render
```
