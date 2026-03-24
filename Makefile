APP_NAME := orcas
VERSION ?= 0.1.0
TARGET ?= x86_64-unknown-linux-gnu

PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin
LIBEXECDIR := $(PREFIX)/libexec/$(APP_NAME)
SHAREDIR := $(PREFIX)/share/$(APP_NAME)

SYSTEMD_DIR ?= $(HOME)/.config/systemd/user

DIST_NAME := $(APP_NAME)-v$(VERSION)-$(TARGET)
DIST_DIR := dist/$(DIST_NAME)

CARGO := cargo
E2E_RUNNER := tests/e2e/run_all.sh
SCENARIO ?=
TAG ?=

MAIN_BIN := orcas
AUX_BINS := orcasd
ALL_BINS := $(MAIN_BIN) $(AUX_BINS)

RELEASE_DIR := target/$(TARGET)/release

.PHONY: all
all: build

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: clippy
clippy:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: test
test:
	$(CARGO) test --workspace

.PHONY: test-e2e
test-e2e:
	@E2E_SUITE=deterministic $(if $(SCENARIO),E2E_SCENARIO=$(SCENARIO),) $(if $(TAG),E2E_TAG=$(TAG),) ./$(E2E_RUNNER)

.PHONY: test-e2e-live
test-e2e-live:
	@E2E_SUITE=live $(if $(SCENARIO),E2E_SCENARIO=$(SCENARIO),) $(if $(TAG),E2E_TAG=$(TAG),) ./$(E2E_RUNNER)

.PHONY: test-e2e-long
test-e2e-long:
	@E2E_SUITE=long $(if $(SCENARIO),E2E_SCENARIO=$(SCENARIO),) $(if $(TAG),E2E_TAG=$(TAG),) ./$(E2E_RUNNER)

.PHONY: build
build:
	$(CARGO) build --workspace --release --target $(TARGET)

.PHONY: check
check:
	$(CARGO) check --workspace

.PHONY: doc
doc:
	$(CARGO) doc --workspace --no-deps

.PHONY: install
install: build
	install -d "$(DESTDIR)$(BINDIR)"
	install -m 0755 "$(RELEASE_DIR)/$(MAIN_BIN)" "$(DESTDIR)$(BINDIR)/$(MAIN_BIN)"
	install -m 0755 "$(RELEASE_DIR)/orcasd" "$(DESTDIR)$(BINDIR)/orcasd"

.PHONY: install-user
install-user:
	$(MAKE) install PREFIX="$(HOME)/.local"

.PHONY: install-systemd
install-systemd:
	install -d "$(DESTDIR)$(SYSTEMD_DIR)"
	sed 's|^ExecStart=.*|ExecStart=$(BINDIR)/orcasd|' \
		packaging/systemd/orcas-daemon.service \
		> "$(DESTDIR)$(SYSTEMD_DIR)/orcas-daemon.service"
	chmod 0644 "$(DESTDIR)$(SYSTEMD_DIR)/orcas-daemon.service"

.PHONY: enable-systemd
enable-systemd:
	systemctl --user daemon-reload
	systemctl --user enable --now orcas-daemon.service

.PHONY: disable-systemd
disable-systemd:
	systemctl --user disable --now orcas-daemon.service || true

.PHONY: uninstall
uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/orcas"
	rm -f "$(DESTDIR)$(BINDIR)/orcasd"

.PHONY: uninstall-systemd
uninstall-systemd:
	rm -f "$(DESTDIR)$(SYSTEMD_DIR)/orcas-daemon.service"

.PHONY: dist
dist: build
	rm -rf "$(DIST_DIR)"
	install -d "$(DIST_DIR)/bin"
	install -d "$(DIST_DIR)/packaging/systemd"
	install -m 0755 "$(RELEASE_DIR)/orcas" "$(DIST_DIR)/bin/orcas"
	install -m 0755 "$(RELEASE_DIR)/orcasd" "$(DIST_DIR)/bin/orcasd"
	install -m 0644 packaging/systemd/orcas-daemon.service \
		"$(DIST_DIR)/packaging/systemd/orcas-daemon.service"
	test ! -f README.md || install -m 0644 README.md "$(DIST_DIR)/README.md"
	test ! -f LICENSE || install -m 0644 LICENSE "$(DIST_DIR)/LICENSE"
	cd dist && tar -czf "$(DIST_NAME).tar.gz" "$(DIST_NAME)"

.PHONY: clean
clean:
	$(CARGO) clean

.PHONY: clean-e2e
clean-e2e:
	rm -rf target/e2e

.PHONY: clean-dist
clean-dist:
	rm -rf dist


# Local GPT-OSS 20B via vLLM 

# ── Images ──────────────────────────────────────────────
IMAGE := vllm/vllm-openai:v0.18.0
OPEN_WEBUI_IMAGE := ghcr.io/open-webui/open-webui:main

# ── Model / paths ───────────────────────────────────────
MODEL_PATH ?= /data/models/openai/gpt-oss-20b
CACHE_PATH := $(HOME)/.cache/vllm

# ── Networking / ports ──────────────────────────────────
PORT ?= 8000
DOCKER_NETWORK ?= local-inference-net
DOCKER_RESTART_POLICY ?= no
DOCKER_PULL_POLICY ?= never

# ── Container names ─────────────────────────────────────
LLM_CONTAINER ?= vllm-gpt-oss-20b
CONTAINERS := $(LLM_CONTAINER) 

# ── GPU pinning ─────────────────────────────────────────
# GPU=0, GPU=1, or GPU=all
GPU ?= 1

ifeq ($(GPU),all)
  GPU_FLAG := --gpus all
else
  GPU_FLAG := --gpus '"device=$(GPU)"'
endif

# ── vLLM defaults ───────────────────────────────────────
# Overridden by the run-gpt-* profile targets below.
GPU_MEM_UTIL ?= 0.92
MAX_MODEL_LEN ?= 16384
TP_SIZE ?= 1
EXTRA_ARGS ?= --max-num-seqs 4 --max-num-batched-tokens 8192
SERVED_MODEL_NAME ?= gpt-oss-20b

# ── Open WebUI settings ─────────────────────────────────

.PHONY: run-llm run-gpt-balanced run-gpt-32k run-gpt-64k-kvfp8 \
	stop stop-all stop-llm \
	logs logs-llm \
	healthcheck healthcheck-llm \
	models shell clean clean-all \
	smoke-single smoke-batch smoke

# ── Run: vLLM server ────────────────────────────────────
run-llm: stop-llm 
	docker network inspect $(DOCKER_NETWORK) >/dev/null 2>&1 || docker network create $(DOCKER_NETWORK)
	docker run -d \
		--name $(LLM_CONTAINER) \
		--network $(DOCKER_NETWORK) \
		$(GPU_FLAG) \
		--ipc=host \
		-p $(PORT):8000 \
		-v $(MODEL_PATH):/model:ro \
		-v $(CACHE_PATH):/root/.cache \
		--restart $(DOCKER_RESTART_POLICY) \
		--pull $(DOCKER_PULL_POLICY) \
		$(IMAGE) \
		/model \
		--served-model-name $(SERVED_MODEL_NAME) \
		--host 0.0.0.0 \
		--port 8000 \
		--gpu-memory-utilization $(GPU_MEM_UTIL) \
		--max-model-len $(MAX_MODEL_LEN) \
		--tensor-parallel-size $(TP_SIZE) \
		--enable-prefix-caching \
		$(EXTRA_ARGS)
	docker logs -f $(LLM_CONTAINER)


# ── GPT profiles ────────────────────────────────────────
# Good everyday single-3090 default
run-gpt-balanced:
	$(MAKE) run-llm \
		GPU=1 \
		TP_SIZE=1 \
		GPU_MEM_UTIL=0.92 \
		MAX_MODEL_LEN=16384 \
		EXTRA_ARGS="--max-num-seqs 4 --max-num-batched-tokens 4096"

run-gpt-parallel:
	$(MAKE) run-llm \
		GPU=1 \
		TP_SIZE=1 \
		GPU_MEM_UTIL=0.92 \
		MAX_MODEL_LEN=16384 \
		EXTRA_ARGS="--max-num-seqs 8 --max-num-batched-tokens 8192"

run-gpt-32k:
	$(MAKE) run-llm \
		GPU=1 \
		TP_SIZE=1 \
		GPU_MEM_UTIL=0.92 \
		MAX_MODEL_LEN=32768 \
		EXTRA_ARGS="--max-num-seqs 2 --max-num-batched-tokens 4096"

run-gpt-dual-fastest:
	$(MAKE) run-llm \
		GPU=all \
		TP_SIZE=2 \
		GPU_MEM_UTIL=0.88 \
		MAX_MODEL_LEN=16384 \
		EXTRA_ARGS="--max-num-seqs 4 --max-num-batched-tokens 4096"

run-gpt-dual-fast:
	$(MAKE) run-llm \
		GPU=all \
		TP_SIZE=2 \
		GPU_MEM_UTIL=0.88 \
		MAX_MODEL_LEN=16384 \
		EXTRA_ARGS="--max-num-seqs 6 --max-num-batched-tokens 4096"

run-gpt-dual-long:
	$(MAKE) run-llm \
		GPU=all \
		TP_SIZE=2 \
		GPU_MEM_UTIL=0.92 \
		MAX_MODEL_LEN=65536 \
		EXTRA_ARGS="--max-num-seqs 1 --max-num-batched-tokens 8192"

run-gpt-dual-longest:
	$(MAKE) run-llm \
		GPU=all \
		TP_SIZE=2 \
		GPU_MEM_UTIL=0.94 \
		MAX_MODEL_LEN=98304 \
		EXTRA_ARGS="--max-num-seqs 1 --max-num-batched-tokens 8192"

# ── Stop ────────────────────────────────────────────────
stop: stop-llm

stop-all:
	@for c in $(CONTAINERS); do \
		docker stop $$c 2>/dev/null || true; \
		docker rm $$c 2>/dev/null || true; \
	done
	@echo "All containers stopped"

stop-llm:
	docker stop $(LLM_CONTAINER) 2>/dev/null || true
	docker rm $(LLM_CONTAINER) 2>/dev/null || true		

# ── Logs ────────────────────────────────────────────────
logs:
	@for c in $(CONTAINERS); do \
		if docker ps --format '{{.Names}}' | grep -q "^$$c$$"; then \
			docker logs -f $$c; \
			exit 0; \
		fi; \
	done; \
	echo "No running container found"

logs-llm:
	docker logs -f $(LLM_CONTAINER)

# ── Healthchecks ────────────────────────────────────────
healthcheck: healthcheck-llm

healthcheck-llm:
	@curl -sf http://localhost:$(PORT)/health && echo "healthy" || echo "unhealthy"

# ── Utility ─────────────────────────────────────────────
models:
	@curl -s http://localhost:$(PORT)/v1/models | python3 -m json.tool

SMOKE_ENDPOINT ?= http://localhost:8000/v1/responses
SMOKE_OUT ?= .respkit_smoke
SMOKE_INPUT_DIR ?= tests/fixtures/rename_inputs
SMOKE_INPUT_FILE ?= $(SMOKE_INPUT_DIR)/clean_easy.txt
SMOKE_MAX_CONCURRENCY ?= 1
SMOKE_REVIEW_MAX_CONCURRENCY ?= 1
SMOKE_PROVIDER_TIMEOUT ?= 30
SMOKE_REVIEW ?=

smoke-single:
	@rm -rf $(SMOKE_OUT)
	@mkdir -p $(SMOKE_OUT)
	@python3 -m examples.demo_rename_proposal single $(SMOKE_INPUT_FILE) --endpoint $(SMOKE_ENDPOINT) --out $(SMOKE_OUT) --provider-timeout $(SMOKE_PROVIDER_TIMEOUT) $(if $(SMOKE_REVIEW),--review,)

smoke-batch:
	@rm -rf $(SMOKE_OUT)
	@mkdir -p $(SMOKE_OUT)
	@python3 -m examples.demo_rename_proposal batch $(SMOKE_INPUT_DIR) --endpoint $(SMOKE_ENDPOINT) --out $(SMOKE_OUT) --max-concurrency $(SMOKE_MAX_CONCURRENCY) --review-max-concurrency $(SMOKE_REVIEW_MAX_CONCURRENCY) --provider-timeout $(SMOKE_PROVIDER_TIMEOUT) $(if $(SMOKE_REVIEW),--review,)

smoke:
	@$(MAKE) smoke-single
	@$(MAKE) smoke-batch

shell:
	@for c in $(CONTAINERS); do \
		if docker ps --format '{{.Names}}' | grep -q "^$$c$$"; then \
			docker exec -it $$c /bin/bash; \
			exit 0; \
		fi; \
	done; \
	echo "No running container found"

# ── Clean ───────────────────────────────────────────────
clean: stop-llm
	docker rmi $(IMAGE) 2>/dev/null || true

clean-all: stop-all
	docker rmi $(IMAGE) 2>/dev/null || true
