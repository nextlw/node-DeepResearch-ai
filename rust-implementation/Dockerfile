# ==============================================================================
# DOCKERFILE - DEEP RESEARCH RUST SERVER (Railway Deploy)
# ==============================================================================
# Otimizações:
# 1. cargo-chef para cache de dependências (rebuild rápido ~2min)
# 2. Multi-stage build: planner → builder → runtime
# 3. debian:trixie-slim para runtime (compatível com glibc 2.39 do rust:1.90)
# 4. Imagem final ~100MB (sem toolchain Rust)
# ==============================================================================

# ------------------------------------------------------------------------------
# STAGE 1: PLANNER - Gera "receita" de dependências com cargo-chef
# ------------------------------------------------------------------------------
FROM rust:1.90-slim AS planner

RUN cargo install cargo-chef --locked

WORKDIR /app

# Copia manifests para calcular dependências
COPY Cargo.toml Cargo.lock ./
COPY benches/ benches/

# Cria src dummy para cargo-chef
RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs && \
    echo '' > src/lib.rs

# Gera receita de dependências
RUN cargo chef prepare --recipe-path recipe.json

# ------------------------------------------------------------------------------
# STAGE 2: BUILDER - Compila dependências (cache) + código fonte
# ------------------------------------------------------------------------------
FROM rust:1.90-slim AS builder

RUN cargo install cargo-chef --locked && \
    apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copia receita do planner
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/Cargo.toml Cargo.toml
COPY --from=planner /app/Cargo.lock Cargo.lock
COPY benches/ benches/

# ============================================================================
# CACHE: Compila APENAS dependências (~10min, mas fica em cache)
# ============================================================================
RUN cargo chef cook --release --features server --recipe-path recipe.json

# Copia código-fonte real
COPY src/ src/

# ============================================================================
# BUILD: Compila apenas o código (~1-2min com deps em cache)
# ============================================================================
RUN cargo build --release --features server

# ------------------------------------------------------------------------------
# STAGE 3: RUNTIME - Imagem final mínima
# NOTA: debian:trixie-slim para compatibilidade com glibc 2.39 (rust:1.90)
# ------------------------------------------------------------------------------
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Usuário não-root para segurança
RUN useradd -r -u 1000 app

WORKDIR /app
RUN chown app:app /app

# Copia binário compilado
COPY --from=builder /app/target/release/deep-research-cli .
RUN chmod +x /app/deep-research-cli && chown app:app /app/deep-research-cli

USER app

EXPOSE 3000

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s CMD curl -f http://localhost:${PORT:-3000}/health || exit 1

# Railway injeta PORT automaticamente
CMD ["/bin/sh", "-c", "/app/deep-research-cli --server --port=${PORT:-3000}"]
