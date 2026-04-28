# Kronforce Makefile
#
# Usage:
#   make dev          — run locally
#   make build        — cargo build (debug)
#   make release      — cargo build (release)
#   make test         — run all tests (Rust + JS)
#   make lint         — clippy + fmt check
#   make fix          — auto-fix clippy + fmt
#   make docker       — build Docker image for linux/amd64
#   make push         — build + push Docker image to GHCR
#   make deploy       — push image, then tell droplet to pull
#   make seed-local   — seed local instance with demo data
#   make clean        — cargo clean

IMAGE := ghcr.io/mikemiles-dev/kronforce
TAG := latest

.PHONY: dev build release test lint fix docker push deploy seed-local clean

# --- Development ---

dev:
	cargo run --bin kronforce

build:
	cargo build

release:
	cargo build --release

# --- Quality ---

test:
	cargo test --all
	@echo "--- JS Tests ---"
	@for f in web/tests/test_*.js; do node "$$f" 2>&1 | tail -1; done

lint:
	cargo clippy --all-targets
	cargo fmt --all -- --check
	cargo deny check

fix:
	cargo clippy --fix --allow-dirty
	cargo fmt

# --- Docker ---

docker:
	docker build --platform linux/amd64 -t $(IMAGE):$(TAG) -f deploy/docker/Dockerfile .

push: docker
	docker push $(IMAGE):$(TAG)
	@echo ""
	@echo "Pushed $(IMAGE):$(TAG)"
	@echo "Droplet cron will pick this up within the hour."
	@echo "To update now: ssh root@your-droplet 'cd ~/kronforce-platform/deploy && docker compose pull demo && docker compose up -d demo'"

deploy: push
	@echo "Pulling on droplet..."
	ssh root@kronforce-prod 'cd ~/kronforce-platform/deploy && docker compose pull demo && docker compose up -d demo'
	@echo "Demo updated."

# --- Seed ---

seed-local:
	@echo "Seeding localhost:8080..."
	@KEY=$$(cargo run --bin kronforce 2>&1 | grep -oE 'kf_[A-Za-z0-9+/=]+' | head -1); \
	if [ -z "$$KEY" ]; then echo "Start the server first: make dev"; exit 1; fi; \
	KRONFORCE_URL=http://localhost:8080 ADMIN_KEY="$$KEY" sh demo/seed-demo.sh

# --- Cleanup ---

clean:
	cargo clean
