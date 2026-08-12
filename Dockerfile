FROM node:lts-alpine AS frontend-builder

WORKDIR /frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npm run build

FROM rust:bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations

# Esta camada mantém as dependências compiladas quando apenas o código muda.
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs && cargo build --locked --release

COPY src ./src
RUN touch src/main.rs && cargo build --locked --release && \
    cp /build/target/release/agendarx /tmp/agendarx

FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install --no-install-recommends -y ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --system agendarx && \
    useradd --system --gid agendarx --home-dir /app agendarx && \
    mkdir -p /app/data && \
    chown -R agendarx:agendarx /app

WORKDIR /app
COPY --from=builder --chown=agendarx:agendarx /tmp/agendarx /usr/local/bin/agendarx
COPY --from=frontend-builder --chown=agendarx:agendarx /frontend/dist /app/frontend

USER agendarx
ENV SERVER_ADDR=0.0.0.0:3000 \
    DATABASE_URL=sqlite://data/agendarx.db?mode=rwc \
    FRONTEND_DIR=/app/frontend \
    RUST_LOG=agendarx=info,tower_http=info
VOLUME ["/app/data"]
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/agendarx"]
