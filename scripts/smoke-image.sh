#!/usr/bin/env bash
set -euo pipefail

image="${1:?Informe a imagem publicada com digest}"
test_container_id=""
cleanup() {
  if [[ -n "$test_container_id" ]]; then docker stop "$test_container_id" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

for platform in linux/amd64 linux/arm64 linux/arm/v7; do
  test_container_id=$(docker run --rm -d --platform "$platform" \
    -p 127.0.0.1::12000 \
    -e JWT_SECRET=teste-de-publicacao-segredo-temporario-com-mais-de-32-caracteres \
    -e ADMIN_LOGIN=teste -e ADMIN_PASSWORD=teste-de-publicacao-local \
    "$image")
  mapping=$(docker port "$test_container_id" 12000/tcp)
  port="${mapping##*:}"
  ready=false
  for attempt in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:$port/health" 2>/dev/null | python3 -c 'import json,sys; assert json.load(sys.stdin)["status"] == "ok"' 2>/dev/null; then
      ready=true
      break
    fi
    sleep 1
  done
  if [[ "$ready" != true ]]; then
    docker logs "$test_container_id" 2>&1 || true
    echo "Falha de inicialização em $platform" >&2
    exit 1
  fi
  echo "Health OK: $platform"
  cleanup
  test_container_id=""
done
