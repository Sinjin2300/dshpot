# DeadSimpleHoneypot - dshpot

An SSH honeypot written in Rust. It listens for incoming SSH connections,
records authentication attempts to a SQLite database, and exposes metrics on
attacker behaviour.

Connections are never granted access. The server presents itself as a standard
OpenSSH instance and allows repeated password attempts, logging every username
and password pair it receives.

## Features

- Records all inbound connections and authentication attempts to SQLite
- Mimics an OpenSSH 9.6 server identity
- Dual-channel structured logging — app health to stderr, runtime events to
  stdout (optionally as JSON for log ingestion pipelines)
- Prometheus metrics exporter (in progress)
- Nix flake with naersk for reproducible builds and a container image target

## Nix flake quickstart

```sh
nix run "github:Sinjin2300/dshpot" -- init && 
nix run "github:Sinjin2300/dshpot" -- serve
```

Now you will have the database as well as the host key in whichever directory
that you ran the command in.

## Usage

Before running the server for the first time, initialise the database and
generate a host key:

```sh
dshpot init
```

This creates `honeypot.db` and `honeypot_host_key.pem` in the current directory.
Paths can be overridden:

```sh
dshpot init --db /var/lib/dshpot/honeypot.db --host-key /var/lib/dshpot/host_key.pem
```

Start the server:

```sh
dshpot serve
```

Full options:

```
dshpot serve
  -b, --bind-ip <IP>              Bind address (default: 0.0.0.0)
  -p, --port <PORT>               Listen port, must be >= 1024 (default: 2222)
  -d, --db <PATH>                 Path to SQLite database (default: honeypot.db)
  -k, --host-key <PATH>           Path to host key PEM (default: honeypot_host_key.pem)
      --metrics-exporter <TYPE>   Metrics backend: prometheus | file
      --prom-ip <IP>              Prometheus exporter bind address
      --prom-port <PORT>          Prometheus exporter port
      --metrics-dir <PATH>        File path to directory to be populated with metrics
  -l, --log-level <LEVEL>         Log level: trace | debug | info | warn | error (default: warn)
  -j, --output-json               Emit runtime events as JSON on stdout
```

## Building

Build the binary:

```sh
nix build
```

Build a container image:

```sh
nix build .#container
```

Run the container image (with some opinionated options):

```sh
{podman/docker} run \
--name dshpot \
--restart unless-stopped \
-e 'LOG_LEVEL=info' \
-e 'BIND_PORT=2222' \
-e 'BIND_IP=0.0.0.0' \
-p 2222:2222 \
-v /tmp/dshpot:/data \
docker-archive:./result \
--metrics-exporter file
```

With Cargo directly:

```sh
cargo build --release
```

## Database Schema

Two tables are created on `init`:

`connections` — one row per TCP connection, recording source/destination IP and
port, timestamp, and connection duration.

`auth_attempts` — one row per authentication attempt, linked to a connection by
foreign key, recording username, password, attempt number, and method.

For the full schema, refer to `./src/schema.sql`

## Logging

Runtime events (connections, auth attempts) are written to stdout. Application
health events (startup, errors, warnings) are written to stderr. This separation
is intentional — it allows stdout to be piped into a log aggregator without
mixing operational noise.

Pass `-j` to emit runtime events as structured JSON:

```sh
dshpot serve -j | jq .
```

## TODO

- [ ] Implement Prometheus metrics exporter (`metrics.rs` is currently stubbed)
- [x] Implement file-based metrics export
- [ ] Switch raw `sqlx::query` calls to `sqlx::query!` macros for compile-time
      SQL verification
- [ ] Capture SSH client version string and store in `client_version` column
- [ ] Integration tests against the library crate
- [x] Make it reference proper data dir for files (env var DATA_DIR)
- [ ] NixOS module for running as a systemd service with proper state directory
