# Personal Blog System

A modern blog deployment system featuring a Hugo static site generator, Rust backend with like functionality, and comprehensive observability stack. Built with Nix flakes for reproducible deployments and NixOS for production hosting.

## Architecture Overview

This system consists of three main components orchestrated through Nix flakes:

1. **Static Blog** (`blog-static/`) - Hugo-powered site using hugo-coder theme with interactive like functionality
2. **Backend** (`backend/`) - Rust web server providing like API endpoints with PostgreSQL database and observability 
3. **Infrastructure** - NixOS-based deployment with OpenTelemetry Collector and Coralogix integration

## Prerequisites

- **Nix with flakes enabled** (recommended: use the Determinate Nix Installer)
- **PostgreSQL** (for local development)
- **Docker** (optional, for containerized PostgreSQL)

## Quick Start

### 1. Setup Development Environment

```bash
# Clone the repository
git clone <repository-url>
cd blog

# Enter development shell with all dependencies
nix develop

# This provides: Hugo, OpenTofu, SQLx CLI, Rust toolchain, PostgreSQL client
```

### 2. Database Setup

#### Option A: Local PostgreSQL Installation

```bash
# Install PostgreSQL (distribution-specific)
# Ubuntu/Debian: sudo apt install postgresql postgresql-contrib
# macOS: brew install postgresql

# Create user and database
sudo -u postgres psql -c "CREATE USER blog WITH PASSWORD 'blog';"
sudo -u postgres psql -c "ALTER USER blog CREATEDB;"
sudo -u postgres createdb -O blog blog
sudo -u postgres psql -d blog -c "GRANT ALL PRIVILEGES ON DATABASE blog TO blog;"
sudo -u postgres psql -d blog -c "GRANT ALL PRIVILEGES ON SCHEMA public TO blog;"

# Run database migrations
cd backend
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" sqlx migrate run
```

#### Option B: Docker PostgreSQL

```bash
# Start PostgreSQL container
docker run -d \
  --name blog-postgres \
  -e POSTGRES_USER=blog \
  -e POSTGRES_PASSWORD=blog \
  -e POSTGRES_DB=blog \
  -p 5432:5432 \
  postgres:15

# Run migrations
cd backend
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" sqlx migrate run
```

### 3. Generate SQLx Offline Data

For offline compilation (required for Nix builds):

```bash
cd backend

# Generate sqlx-data.json (requires database connection)
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" cargo sqlx prepare

# Verify offline compilation works
SQLX_OFFLINE=true cargo check
```

## Building & Testing

### Build All Components

```bash
# Build everything (static site + backend + tests)
nix build

# Build specific components
nix build .#blog-static.packages.x86_64-linux.default  # Static site
nix build .#backend.packages.x86_64-linux.default     # Backend binary
```

### Run Tests

```bash
# Run all checks (backend tests + integration tests + linting)
nix flake check

# Run only backend unit tests
cd backend
cargo test

# Run specific integration test
nix run .#checks.x86_64-linux.integration
```

### Development Servers

#### Hugo Development Server

```bash
cd blog-static
hugo server

# Site available at http://localhost:1313
```

#### Backend Development Server

```bash
cd backend

# Set environment variables
export DATABASE_URL="postgresql://blog:blog@localhost:5432/blog"
export RUST_LOG="info"

# Create test blog posts JSON (temporary file for development)
echo '[{"title":"Test Post","slug":"test-post","description":"A test post","date":"2024-01-01T12:00:00Z","featuredImage":null,"tags":["test"],"url":"https://blog.flakm.com/posts/test-post"}]' > /tmp/posts.json

# Run backend server
cargo run /tmp/posts.json

# Backend available at http://localhost:3000
# Health check: curl http://localhost:3000/health
# Like a post: curl -X POST http://localhost:3000/like/test-post
# Get likes: curl http://localhost:3000/likes/test-post
```

## API Endpoints

The backend provides the following REST API endpoints:

- `GET /health` - Health check endpoint
- `POST /like/{post-slug}` - Like a blog post (rate limited: 1 per hour per IP)
- `GET /likes/{post-slug}` - Get like count for a blog post
- `GET /metrics` - Prometheus metrics (if enabled)

### Rate Limiting

The like functionality includes built-in rate limiting:
- **1 like per post per IP address per hour**
- Duplicate likes within the time window return a rate limit message
- Rate limiting is implemented using PostgreSQL unique constraints

### Response Format

```json
{
  "success": true|false,
  "message": "Response message",
  "total_likes": 42
}
```

## Database Schema

The system uses PostgreSQL with the following main tables:

- `blog_posts` - Blog post metadata loaded from Hugo JSON export
- `blog_post_likes` - Like tracking with IP-based rate limiting

Migration files are located in `backend/migrations/` and are automatically applied on startup.

## Configuration

### Hugo Configuration (`blog-static/config.toml`)

```toml
[params.likes]
  enable = true
  apiBase = "https://fedi.flakm.com"  # Backend API URL

[params.plausibleAnalytics]
  domain = "flakm.com"
  serverURL = "plausible.flakm.com"
```

### Environment Variables

Backend configuration via environment variables:

```bash
# Database
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog"

# Logging
RUST_LOG="info"  # debug, info, warn, error

# OpenTelemetry (optional)
OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"
OTEL_SERVICE_NAME="blog-backend"
OTEL_SERVICE_VERSION="1.0.0"
OTEL_RESOURCE_ATTRIBUTES="deployment.environment=development"

# Server
BIND_ADDRESS="127.0.0.1:3000"  # Optional, defaults to 127.0.0.1:3000
```

## Nix Integration Tests

The system includes comprehensive integration tests that run in KVM virtual machines using the NixOS testing framework.

### Running Integration Tests

```bash
# Run integration tests (creates VM, tests all components)
nix run .#checks.x86_64-linux.integration

# Run integration tests in interactive mode (for debugging)
nix run -L .#checks.x86_64-linux.integration.driverInteractive
```

### What the Integration Tests Cover

The integration tests (`backend/nixos-test.nix`) verify:

1. **Service Startup**: PostgreSQL, OpenTelemetry Collector, Backend, Nginx
2. **Database Functionality**: 
   - PostgreSQL service running
   - Blog posts loaded from JSON
   - Database connectivity
3. **API Endpoints**:
   - Health check endpoint
   - Like a post (creates database entry)  
   - Get likes count
   - Error handling for non-existent posts
   - Rate limiting (multiple likes to same post)
4. **Observability Stack**:
   - OpenTelemetry Collector receiving data
   - Metrics endpoint availability
5. **Proxy Configuration**:
   - Nginx reverse proxy working
   - Proper header forwarding

### Test Architecture

The integration tests create two virtual machines:

- **Server VM**: Runs all services (PostgreSQL, Backend, OTEL Collector, Nginx)
- **Client VM**: Makes HTTP requests to test the API

Test execution includes:
- Colored output with termcolor for better readability
- Step-by-step verification with detailed error reporting
- Network connectivity testing between VMs
- Service dependency verification
- End-to-end API functionality testing

### Interactive Testing Mode

For debugging failed tests:

```bash
# Start interactive test environment
nix run -L .#checks.x86_64-linux.integration.driverInteractive

# In the interactive Python REPL:
>>> start_all()
>>> server.wait_for_unit("postgresql.service")
>>> server.succeed("curl http://localhost:3000/health")
>>> client.succeed("curl http://server/health")
```

This allows you to manually inspect the VMs and debug issues.

## Production Deployment

### NixOS Configuration

The system is designed for deployment on NixOS. The main configuration is in `configuration.nix` and includes:

- PostgreSQL 15 with blog user and database
- OpenTelemetry Collector with Coralogix integration
- Nginx reverse proxy with SSL/TLS (ACME)
- Systemd service for the backend
- Firewall configuration for required ports

### Deployment Commands

```bash
# Deploy to production server
nixos-rebuild switch --target-host root@hetzner-blog --flake .#blog

# Infrastructure provisioning (OpenTofu)
tofu init
tofu plan
tofu apply
```

### Observability

The production system includes comprehensive observability:

- **Traces**: Sent to Coralogix via OpenTelemetry Collector
- **Metrics**: Prometheus metrics scraped and forwarded to Coralogix  
- **Logs**: Application logs shipped to Coralogix
- **Service Monitoring**: All services monitored via systemd

## Development Workflow

### Adding New Features

1. **Database Changes**: Add migration in `backend/migrations/`
2. **Backend Changes**: Update Rust code, regenerate SQLx data
3. **Frontend Changes**: Update Hugo templates and CSS
4. **Testing**: Run integration tests to verify everything works
5. **Deployment**: Deploy via NixOS configuration

### SQLx Workflow

When modifying database queries:

```bash
cd backend

# Make your query changes using sqlx! macros
# Then regenerate the offline data
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" cargo sqlx prepare

# Verify offline compilation
SQLX_OFFLINE=true cargo check

# Commit the updated .sqlx/ directory
git add .sqlx/
```

## Troubleshooting

### Common Issues

1. **SQLx offline compilation fails**:
   - Ensure database is running and migrations applied
   - Regenerate sqlx-data.json with `cargo sqlx prepare`

2. **Integration tests fail**:
   - Check if ports 5432, 4317, 4318, 3000, 80 are available
   - Verify PostgreSQL is not running on host (conflicts with VM)

3. **Backend can't connect to database**:
   - Verify DATABASE_URL environment variable
   - Check PostgreSQL service status and permissions

4. **Hugo site builds but likes don't work**:
   - Verify `params.likes.enable = true` in config.toml  
   - Check that backend API is accessible from frontend
   - Inspect browser developer tools for JavaScript errors

### Log Inspection

```bash
# Backend logs (in development)
RUST_LOG=debug cargo run /tmp/posts.json

# SystemD logs (in production)
journalctl -u backend.service -f

# PostgreSQL logs
journalctl -u postgresql.service -f

# OpenTelemetry Collector logs  
journalctl -u opentelemetry-collector.service -f
```

## Manage secrets

Insert yubikey and run:

```bash
nix-shell -p sops --run "sops secrets/secrets.yaml"
```

The asc for the secrets was generated with:

```bash
ssh root@hetzner-blog  "sudo cat /etc/ssh/ssh_host_rsa_key" | nix-shell -p ssh-to-pgp --run "ssh-to-pgp -o server01.asc"
gpg --import server01.asc
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

