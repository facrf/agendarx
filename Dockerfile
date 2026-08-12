# syntax=docker/dockerfile:1.7

ARG BUILDPLATFORM=linux/amd64
FROM --platform=${BUILDPLATFORM} node:lts-alpine@sha256:d32cdf619f63fe0471182d08996dd516c6275bb5fd31ae06e55a570bd9e1ad43 AS frontend-builder

WORKDIR /frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npm run build

FROM rust:bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations

# Esta camada mantém as dependências compiladas quando apenas o código muda.
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs && cargo build --locked --release

COPY src ./src
RUN touch src/main.rs && cargo build --locked --release && \
    cp /build/target/release/agendarx /tmp/agendarx

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS runtime

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
LABEL org.opencontainers.image.source="https://github.com/facrf/agendarx"
ENV SERVER_ADDR=0.0.0.0:12000 \
    DATABASE_URL=sqlite://data/agendarx.db?mode=rwc \
    FRONTEND_DIR=/app/frontend \
    RUST_LOG=agendarx=info,tower_http=info
VOLUME ["/app/data"]
EXPOSE 12000

ENTRYPOINT ["/usr/local/bin/agendarx"]
