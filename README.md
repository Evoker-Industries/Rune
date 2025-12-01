# Rune

🔮 A Docker-like and Docker-compatible container service written in Rust with a Terminal User Interface (TUI).

## Features

- **Docker-compatible CLI** - Familiar commands like `run`, `build`, `ps`, `exec`, etc.
- **Container Management** - Create, start, stop, pause, and remove containers
- **Image Building** - Build images from `Runefile` (or `Dockerfile` for compatibility)
- **Docker Compose Support** - Multi-container orchestration with `compose.yaml`
- **Docker Swarm Support** - Cluster management and service orchestration
- **OCI-Compatible Registry** - Built-in container registry following OCI Distribution Specification
- **Terminal User Interface** - Beautiful TUI for managing containers interactively
- **Volume Management** - Persistent storage for containers
- **Network Management** - Bridge, host, overlay, and other network drivers

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/Evoker-Industries/Rune.git
cd Rune

# Build the project
cargo build --release

# Install the binaries
cargo install --path .
```

## Quick Start

### Run a Container

```bash
# Run a container
rune run nginx:latest --name my-nginx -p 8080:80 -d

# List running containers
rune ps

# Stop a container
rune stop my-nginx

# Remove a container
rune rm my-nginx
```

### Build an Image

Create a `Runefile` (or `Dockerfile`):

```dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y curl

WORKDIR /app

COPY . /app

CMD ["./start.sh"]
```

Build the image:

```bash
rune build -t my-app:latest .
```

### Docker Compose

Create a `compose.yaml`:

```yaml
version: "3.8"
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
  db:
    image: postgres:13
    environment:
      POSTGRES_PASSWORD: secret
```

Start the services:

```bash
rune compose up -d
```

### Docker Swarm

Initialize a swarm:

```bash
# Initialize the swarm
rune swarm init

# Create a service
rune service create --name web --replicas 3 nginx:latest

# Scale the service
rune service scale web=5

# List services
rune service ls
```

### Terminal User Interface

Launch the TUI:

```bash
rune tui
# or
rune-tui
```

The TUI provides an interactive interface for:
- Viewing and managing containers
- Monitoring images, networks, and volumes
- Swarm cluster management

**Keyboard shortcuts:**
- `Tab` / `←` `→` - Switch tabs
- `↑` `↓` / `j` `k` - Navigate lists
- `s` - Start container
- `S` - Stop container
- `r` - Restart container
- `p` - Pause container
- `u` - Unpause container
- `d` / `Del` - Delete container
- `?` / `F1` - Show help
- `q` - Quit

## OCI Registry

Rune includes a built-in OCI-compatible container registry:

```bash
# Start the registry (programmatically)
# The registry runs on port 5000 by default

# Push an image
rune image tag my-app:latest localhost:5000/my-app:latest
rune image push localhost:5000/my-app:latest

# Pull an image
rune image pull localhost:5000/my-app:latest
```

## Commands Reference

### Container Commands

| Command | Description |
|---------|-------------|
| `rune run` | Run a new container |
| `rune create` | Create a container |
| `rune start` | Start a container |
| `rune stop` | Stop a container |
| `rune restart` | Restart a container |
| `rune rm` | Remove a container |
| `rune ps` | List containers |
| `rune logs` | Show container logs |
| `rune exec` | Execute command in container |

### Image Commands

| Command | Description |
|---------|-------------|
| `rune build` | Build an image from Runefile |
| `rune image ls` | List images |
| `rune image pull` | Pull an image |
| `rune image push` | Push an image |
| `rune image rm` | Remove an image |
| `rune image tag` | Tag an image |
| `rune image prune` | Remove unused images |

### Network Commands

| Command | Description |
|---------|-------------|
| `rune network ls` | List networks |
| `rune network create` | Create a network |
| `rune network rm` | Remove a network |
| `rune network connect` | Connect container to network |
| `rune network disconnect` | Disconnect container from network |

### Volume Commands

| Command | Description |
|---------|-------------|
| `rune volume ls` | List volumes |
| `rune volume create` | Create a volume |
| `rune volume rm` | Remove a volume |
| `rune volume prune` | Remove unused volumes |

### Compose Commands

| Command | Description |
|---------|-------------|
| `rune compose up` | Create and start containers |
| `rune compose down` | Stop and remove containers |
| `rune compose ps` | List containers |
| `rune compose logs` | View logs |
| `rune compose build` | Build services |
| `rune compose config` | Validate compose file |

### Swarm Commands

| Command | Description |
|---------|-------------|
| `rune swarm init` | Initialize a swarm |
| `rune swarm join` | Join a swarm |
| `rune swarm leave` | Leave the swarm |
| `rune swarm join-token` | Manage join tokens |

### Service Commands (Swarm Mode)

| Command | Description |
|---------|-------------|
| `rune service ls` | List services |
| `rune service create` | Create a service |
| `rune service update` | Update a service |
| `rune service scale` | Scale a service |
| `rune service rm` | Remove a service |
| `rune service rollback` | Rollback a service |

### Node Commands (Swarm Mode)

| Command | Description |
|---------|-------------|
| `rune node ls` | List nodes |
| `rune node inspect` | Inspect a node |
| `rune node update` | Update a node |
| `rune node promote` | Promote to manager |
| `rune node demote` | Demote to worker |
| `rune node rm` | Remove a node |

## Architecture

Rune is built with a modular architecture:

```
src/
├── container/     # Container management
│   ├── config.rs  # Container configuration
│   ├── lifecycle.rs # Lifecycle management
│   └── runtime.rs # Container runtime
├── image/         # Image management
│   ├── builder.rs # Runefile/Dockerfile parsing
│   ├── registry.rs # Registry client
│   └── store.rs   # Local image storage
├── network/       # Network management
│   ├── bridge.rs  # Bridge networks
│   └── config.rs  # Network configuration
├── storage/       # Storage management
│   └── volume.rs  # Volume management
├── compose/       # Docker Compose
│   ├── config.rs  # Compose configuration
│   ├── parser.rs  # YAML parsing
│   └── orchestrator.rs # Service orchestration
├── swarm/         # Docker Swarm
│   ├── cluster.rs # Cluster management
│   ├── node.rs    # Node management
│   ├── service.rs # Service definitions
│   └── task.rs    # Task management
├── registry/      # OCI Registry
│   ├── server.rs  # Registry server
│   ├── storage.rs # Blob storage
│   └── auth.rs    # Authentication
├── tui/           # Terminal UI
│   └── app.rs     # TUI application
└── main.rs        # CLI entry point
```

## Configuration

Rune stores its data in:
- Linux: `~/.local/share/rune/`
- macOS: `~/Library/Application Support/rune/`
- Windows: `C:\Users\<User>\AppData\Local\rune\`

## Docker Compatibility

Rune aims to be compatible with Docker's CLI and APIs:

- ✅ Docker CLI compatible commands
- ✅ Dockerfile support (via Runefile)
- ✅ docker-compose.yml support
- ✅ Docker Swarm mode
- ✅ OCI Image Specification
- ✅ OCI Distribution Specification (Registry)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Docker](https://www.docker.com/) for the inspiration
- [OCI](https://opencontainers.org/) for container standards
- [Ratatui](https://ratatui.rs/) for the TUI framework