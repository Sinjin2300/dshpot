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
- Nix flake with crane for reproducible builds and a container image target
- NixOS module for running as a managed systemd service

## NixOS module

A NixOS module is provided for running dshpot as a managed systemd service with
proper sandboxing and state directory management.

Add dshpot to your system flake inputs:

```nix
inputs = {
  dshpot = {
    url = "github:Sinjin2300/dshpot";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

Import the module in your `nixosSystem`:

```nix
outputs = { nixpkgs, dshpot, ... }: {
  nixosConfigurations.mymachine = nixpkgs.lib.nixosSystem {
    system = "x86_64-linux";
    modules = [
      dshpot.nixosModules.default
      ./configuration.nix
    ];
  };
};
```

Then enable it in your `configuration.nix`:

```nix
services.dshpot = {
  enable = true;
  openFirewall = true;   # opens honeypotPort in the firewall
  honeypotPort = 2222;
  honeypotIp = "0.0.0.0";
  metricsType = "file";  # or "none"
  logLevel = "info";     # trace | debug | info | warn | error
};
```

State is stored in `/var/lib/dshpot` and managed by systemd. The service runs
under a transient `DynamicUser` with a restricted capability set.

### Module options

| Option         | Type    | Default       | Description                                      |
| -------------- | ------- | ------------- | ------------------------------------------------ |
| `enable`       | bool    | `false`       | Enable the dshpot service                        |
| `package`      | package | flake default | Override the dshpot package                      |
| `honeypotPort` | port    | `2222`        | Port the honeypot listens on                     |
| `honeypotIp`   | str     | `"0.0.0.0"`   | IP address to bind to                            |
| `openFirewall` | bool    | `false`       | Open `honeypotPort` in the firewall              |
| `metricsType`  | enum    | `"file"`      | Metrics backend: `file` or `none`                |
| `logLevel`     | enum    | `"warn"`      | Log level: `trace` `debug` `info` `warn` `error` |

## Container

Pre-built images are published to the GitHub Container Registry on every push to
`main`:

```sh
docker pull ghcr.io/sinjin2300/dshpot:latest
```

Run the container:

```sh
docker run \
  --name dshpot \
  --restart unless-stopped \
  -e LOG_LEVEL=info \
  -e BIND_PORT=2222 \
  -e BIND_IP=0.0.0.0 \
  -p 2222:2222 \
  -v /var/lib/dshpot:/data \
  ghcr.io/sinjin2300/dshpot:latest
```

The `/data` volume is where the database and host key are stored. Mount a
persistent volume there to survive container restarts.

### Environment variables

| Variable           | Default   | Description                                             |
| ------------------ | --------- | ------------------------------------------------------- |
| `BIND_IP`          | `0.0.0.0` | IP address to bind to                                   |
| `BIND_PORT`        | `2222`    | Port to listen on                                       |
| `DATA_DIR`         | `/data`   | Directory for database,host key and file metrics if set |
| `LOG_LEVEL`        | `warn`    | Log level                                               |
| `METRICS_EXPORTER` | —         | Metrics backend: `file` or unset                        |

### Building the image locally

If you have Nix installed you can build the image yourself:

```sh
nix build .#container
docker load < result
```

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
- [x] NixOS module for running as a systemd service with proper state directory
