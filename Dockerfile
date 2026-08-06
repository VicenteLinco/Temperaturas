# ==============================================================================
# Multi-Stage Dockerfile for Rust (Axum + SQLx + Postgres)
# Optimizado para Render.com - Reduce la descarga e imagen final a ~35 MB
# ==============================================================================

# --- Stage 1: Build ---
# Requisitos reales de la compilación:
#  - Cargo.lock está en formato v4  -> Cargo >= 1.78 (con 1.77 fallaba con
#    "lock file version `4` was found").
#  - idna_adapter (dependencia transitiva) usa edition2024 -> Rust >= 1.85.
# Se fija una versión concreta para que el build siga siendo reproducible.
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /usr/src/app

# Instalar dependencias de compilación de C / OpenSSL
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copiar archivos de proyecto
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY public ./public

# Compilar binario en modo release con optimizaciones (strip habilitado)
RUN cargo build --release

# --- Stage 2: Runtime Image ---
FROM debian:bookworm-slim AS runner

WORKDIR /app

# Instalar librerías mínimas necesarias para ejecución en producción
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Configurar zona horaria por defecto
ENV TZ=America/Santiago

# Copiar binario compilado y archivos estáticos
COPY --from=builder /usr/src/app/target/release/sistema-temperaturas /usr/local/bin/sistema-temperaturas
COPY --from=builder /usr/src/app/public ./public

# Puerto por defecto para Render
ENV PORT=3000
EXPOSE 3000

# Usuario no privilegiado para seguridad
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

CMD ["sistema-temperaturas"]
