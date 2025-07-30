# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Architecture Overview

This is a personal blog deployment repository with three main components:

1. **Static Blog** (`blog-static/`) - Hugo-powered static site using the hugo-coder theme
2. **Backend** (`backend/`) - Rust web server providing like functionality for blog posts  
3. **Infrastructure** - NixOS-based deployment using OpenTofu for provisioning

The project uses Nix flakes for reproducible builds and development environments, with the root flake orchestrating all components.

## Development Commands

### Building and Development
- `nix develop` - Enter development shell with all required dependencies
- `nix build` - Build all components
- `nix fmt` - Format Nix files
- `nix flake check` - Run all checks including backend tests and integration tests

### Hugo Static Site (blog-static/)
- `hugo server` - Start development server (run from blog-static/ directory)
- `hugo` - Build static site

### Rust Backend (backend/)
- `cargo build` - Build the backend
- `cargo test` - Run backend tests
- `cargo run` - Run the backend server
- `sqlx migrate run` - Apply database migrations

### Infrastructure
- `tofu init` - Initialize OpenTofu
- `tofu plan` - Plan infrastructure changes
- `tofu apply` - Apply infrastructure changes
- `nixos-rebuild switch --target-host root@hetzner-blog --flake .#blog` - Deploy NixOS configuration

## Key Architecture Notes

### Nix Flake Structure
- Root flake coordinates three sub-flakes: backend, blog-static, and infrastructure
- Integration tests run in KVM using NixOS test framework
- Development shell includes Hugo, OpenTofu, SQLx CLI, and Python tools

### Backend Architecture  
- Web server built with Axum web framework
- PostgreSQL database with SQLx for persistence
- Provides REST API endpoints for blog post like functionality
- Includes rate limiting and observability features

### Static Site
- Hugo static site generator with hugo-coder theme
- Content in Markdown format under `content/posts/`
- Custom CSS and layouts for personalization
- Plausible analytics integration

### Database
- PostgreSQL database with migrations in `backend/migrations/`
- Environment variables: `DATABASE_URL=postgresql://blog:blog@localhost:5432/blog`

### Deployment
- NixOS configuration for production deployment
- Disko for disk partitioning
- OpenTofu for cloud infrastructure provisioning on Hetzner
- SSH access via `ssh root@hetzner-blog`