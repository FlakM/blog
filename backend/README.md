# backend

To setup a project run:

```bash
direnv allow
```

## Database Setup

### PostgreSQL Setup (Local)

1. **Install PostgreSQL** (if not already installed)
2. **Create user and database:**
   ```bash
   psql -U postgres -c "CREATE USER blog WITH PASSWORD 'blog';"
   psql -U postgres -c "ALTER USER blog CREATEDB;"
   createdb -U postgres blog
   psql -U postgres -d blog -c "GRANT ALL PRIVILEGES ON DATABASE blog TO blog;"
   psql -U postgres -d blog -c "GRANT ALL PRIVILEGES ON SCHEMA public TO blog;"
   ```

3. **Run migrations:**
   ```bash
   DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" sqlx migrate run
   ```

### PostgreSQL Setup (Docker)

Alternatively, use Docker for PostgreSQL:

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
DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" sqlx migrate run
```

### SQLx Offline Mode

This project uses SQLx offline mode for reproducible builds. If you make changes to database queries:

1. **Generate sqlx-data.json** (requires database connection):
   ```bash
   DATABASE_URL="postgresql://blog:blog@localhost:5432/blog" cargo sqlx prepare
   ```

2. **Verify offline compilation:**
   ```bash
   SQLX_OFFLINE=true cargo check
   ```

## Building and testing



```bash
# build
nix build

# run all tests
nix flake check


# run a specific integration test
nix run .\#checks.x86_64-linux.integration


# run a integration test in an interactive mode
nix run -L .\#checks.x86_64-linux.integration.driverInteractive



```



```bash
curl -H 'Accept: application/activity+json' \
    "https://fedi.flakm.com/blog" | jq

curl -H 'Accept: application/activity+json' "https://fedi.flakm.com/.well-known/webfinger?resource=acct:blog@fedi.flakm.com" | jq
```
