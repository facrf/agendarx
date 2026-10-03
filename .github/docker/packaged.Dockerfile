# syntax=docker/dockerfile:1.7

FROM debian:trixie-slim@sha256:a99cfc517144bc59b1978475ec53b46ecabec7e43635402ee5b77cc54cd1b20a AS runtime

RUN apt-get update && \
    apt-get install --no-install-recommends -y ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --system agendarx && \
    useradd --system --gid agendarx --home-dir /app agendarx && \
    mkdir -p /app/data && \
    chown -R agendarx:agendarx /app

ARG TARGETARCH

WORKDIR /app
COPY --chown=agendarx:agendarx binaries/${TARGETARCH}/agendarx /usr/local/bin/agendarx
COPY --chown=agendarx:agendarx frontend /app/frontend
COPY --chown=agendarx:agendarx docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

USER agendarx
LABEL org.opencontainers.image.source="https://github.com/facrf/agendarx"
ENV SERVER_ADDR=0.0.0.0:12000 \
    DATABASE_URL=sqlite://data/agendarx.db?mode=rwc \
    FRONTEND_DIR=/app/frontend \
    RUST_LOG=agendarx=info,tower_http=info
VOLUME ["/app/data"]
EXPOSE 12000

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
