.PHONY: help build up down restart logs ps clean test dev prod

# Default target
help:
	@echo "ZID Server - Docker Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  help         Show this help message"
	@echo "  build        Build all containers"
	@echo "  up           Start all services"
	@echo "  down         Stop all services"
	@echo "  restart      Restart all services"
	@echo "  logs         Show logs (tail -f)"
	@echo "  ps           Show running containers"
	@echo "  clean        Remove all containers, volumes, and images"
	@echo "  test         Run end-to-end tests"
	@echo "  dev          Start services in development mode"
	@echo "  prod         Build and start in production mode"
	@echo ""
	@echo "Database:"
	@echo "  db-shell     Connect to PostgreSQL shell"
	@echo "  redis-cli    Connect to Redis CLI"
	@echo "  db-migrate   Apply database migrations"
	@echo "  db-reset     Reset database (dangerous!)"
	@echo ""
	@echo "Debug:"
	@echo "  logs-app     Show application logs"
	@echo "  logs-db      Show PostgreSQL logs"
	@echo "  logs-redis   Show Redis logs"
	@echo "  shell        Shell into application container"

# Build containers
build:
	@echo "🔨 Building containers..."
	docker compose build

# Start services
up:
	@echo "🚀 Starting services..."
	docker compose up -d
	@echo "⏳ Waiting for services to be healthy..."
	@sleep 5
	@echo "✅ Services started!"
	@echo ""
	@echo "📊 Service status:"
	@docker compose ps
	@echo ""
	@echo "🌐 Application: http://localhost:5555"
	@echo "🗄️  PostgreSQL: localhost:5432 (user: postgres, db: zid)"
	@echo "🔴 Redis: localhost:6380 (внешний порт, внутри Docker: 6379)"
	@echo ""
	@echo "💡 Run 'make test' to run end-to-end tests"
	@echo "💡 Run 'make logs' to see logs"

# Stop services
down:
	@echo "🛑 Stopping services..."
	docker compose down
	@echo "✅ Services stopped"

# Restart services
restart:
	@echo "🔄 Restarting services..."
	docker compose restart
	@echo "✅ Services restarted"

# Show logs
logs:
	docker compose logs -f

# Show logs for specific services
logs-app:
	docker compose logs -f zid-app

logs-db:
	docker compose logs -f postgres

logs-redis:
	docker compose logs -f redis

# Show running containers
ps:
	docker compose ps

# Clean everything
clean:
	@echo "🧹 Cleaning up..."
	@echo "⚠️  This will remove all containers, volumes, and images!"
	@printf "Are you sure? [y/N] "; \
	read REPLY; \
	case "$$REPLY" in \
		[Yy]*) \
			docker compose down -v --rmi all; \
			echo "✅ Cleanup completed"; \
			;; \
		*) \
			echo "❌ Cleanup cancelled"; \
			;; \
	esac

# Run tests
test:
	@echo "🧪 Running end-to-end tests..."
	./scripts/test.sh

# Development mode (with live reload)
dev:
	@echo "🔧 Starting in development mode..."
	@echo "💡 Code changes will require manual restart"
	docker compose up

# Production mode
prod:
	@echo "🚀 Building and starting in production mode..."
	docker compose build --no-cache
	docker compose up -d
	@echo "✅ Production services started"

# Database shell
db-shell:
	@echo "🗄️  Connecting to PostgreSQL..."
	docker compose exec postgres psql -U postgres -d zid

# Redis CLI
redis-cli:
	@echo "🔴 Connecting to Redis..."
	docker compose exec redis redis-cli

# Apply database migrations
db-migrate:
	@echo "🔄 Applying database migrations..."
	@for f in migrations/*.sql; do \
		echo "  Applying $$f..."; \
		docker compose exec -T postgres psql -U postgres -d zid -f /docker-entrypoint-initdb.d/$$(basename $$f) 2>&1 | grep -v "already exists" || true; \
	done
	@echo "✅ Migrations applied"

# Reset database (dangerous!)
db-reset:
	@echo "⚠️  WARNING: This will delete all data in the database!"
	@printf "Are you sure? [y/N] "; \
	read REPLY; \
	case "$$REPLY" in \
		[Yy]*) \
			docker compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS zid;"; \
			docker compose exec postgres psql -U postgres -c "CREATE DATABASE zid;"; \
			sleep 2; \
			$(MAKE) db-migrate; \
			echo "✅ Database reset completed"; \
			;; \
		*) \
			echo "❌ Reset cancelled"; \
			;; \
	esac

# Shell into application container
shell:
	@echo "🐚 Opening shell in application container..."
	docker compose exec zid-app sh

# Health check
health:
	@echo "🏥 Checking service health..."
	@echo ""
	@echo "PostgreSQL:"
	@docker compose exec -T postgres pg_isready -U postgres || echo "❌ PostgreSQL is down"
	@echo ""
	@echo "Redis:"
	@docker compose exec -T redis redis-cli ping || echo "❌ Redis is down"
	@echo ""
	@echo "Application:"
	@curl -s http://localhost:5555/health > /dev/null && echo "✅ Application is healthy" || echo "❌ Application is down"

# Show environment info
info:
	@echo "ℹ️  ZID Server Information"
	@echo ""
	@echo "Docker Compose:"
	@docker compose version
	@echo ""
	@echo "Running Containers:"
	@docker compose ps
	@echo ""
	@echo "Volumes:"
	@docker volume ls | grep zid
	@echo ""
	@echo "Networks:"
	@docker network ls | grep zid

# Quick start (build + up)
start: build up
	@echo ""
	@echo "🎉 ZID Server is ready!"
	@echo ""
	@echo "🧪 Run end-to-end tests:"
	@echo "   make test"
	@echo ""
	@echo "📝 Or register a user manually:"
	@echo "   curl -X POST http://localhost:5555/register \\"
	@echo "     -H 'Content-Type: application/json' \\"
	@echo "     -d '{\"username\":\"admin\",\"password\":\"secret\"}'"
