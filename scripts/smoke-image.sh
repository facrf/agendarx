#!/usr/bin/env bash
set -euo pipefail

image="${1:?Informe a imagem publicada com digest}"
test_container_name=""
test_process_id=""
cleanup() {
  if [[ -n "$test_container_name" ]]; then docker stop "$test_container_name" >/dev/null 2>&1 || true; fi
  if [[ -n "$test_process_id" ]]; then wait "$test_process_id" 2>/dev/null || true; fi
}
trap cleanup EXIT

mkdir -p .cache
index=$(docker buildx imagetools inspect "$image" --raw)
for platform in linux/amd64 linux/arm64 linux/arm/v7; do
  architecture="${platform#linux/}"
  architecture="${architecture%%/*}"
  manifest_digest=$(jq -r --arg architecture "$architecture" '.manifests[] | select(.platform.os == "linux" and .platform.architecture == $architecture and ($architecture != "arm" or .platform.variant == "v7")) | .digest' <<< "$index")
  [[ "$manifest_digest" =~ ^sha256:[a-f0-9]{64}$ ]]
  # Separate manifests avoid conflicts in Docker's classic image store.
  platform_image="${image%@*}@$manifest_digest"
  test_container_name="agendarx-release-smoke-${architecture}-${BASHPID}"
  log_file=".cache/smoke-${architecture}.log"
  # Attach stdout before startup so crashes remain visible with --rm.
  docker run --rm --name "$test_container_name" --platform "$platform" \
    -p 127.0.0.1::12000 \
    -e JWT_SECRET=teste-de-publicacao-segredo-temporario-com-mais-de-32-caracteres \
    -e ADMIN_LOGIN=teste -e ADMIN_PASSWORD=teste-de-publicacao-local \
    "$platform_image" > "$log_file" 2>&1 &
  test_process_id=$!
  ready=false
  for attempt in $(seq 1 30); do
    mapping=$(docker port "$test_container_name" 12000/tcp 2>/dev/null || true)
    port="${mapping##*:}"
    if [[ -n "$port" ]] && curl -fsS "http://127.0.0.1:$port/health" 2>/dev/null | python3 -c 'import json,sys; assert json.load(sys.stdin)["status"] == "ok"' 2>/dev/null; then
      ready=true
      break
    fi
    if ! kill -0 "$test_process_id" 2>/dev/null; then break; fi
    sleep 1
  done
  if [[ "$ready" != true ]]; then
    cat "$log_file" >&2
    echo "Falha de inicialização em $platform" >&2
    exit 1
  fi
  echo "Health OK: $platform"
  cleanup
  test_container_name=""
  test_process_id=""
done
