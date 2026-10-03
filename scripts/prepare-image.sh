#!/usr/bin/env bash
set -euo pipefail

version="${1:?Informe a versão sem v}"
artifacts="${2:?Informe a pasta dos pacotes}"
context="${3:?Informe a pasta de saída da imagem}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+[a-z]?$ ]] || { echo "Versão inválida" >&2; exit 1; }

# Checksums cover the exact archives produced by the release jobs.
(cd "$artifacts" && sha256sum -c -- *.sha256)
mkdir -p "$context/binaries"
for architecture in amd64 arm64 armv7; do
  package="agendarx-${version}-linux-${architecture}"
  archive="$artifacts/${package}.tar.gz"
  extract="$context/packages/${architecture}"
  mkdir -p "$extract"
  tar -xzf "$archive" -C "$extract"
  image_architecture="$architecture"
  if [[ "$architecture" == armv7 ]]; then image_architecture=arm; fi
  mkdir -p "$context/binaries/$image_architecture"
  install -m 0755 "$extract/$package/agendarx" "$context/binaries/$image_architecture/agendarx"
  if [[ "$architecture" == amd64 ]]; then
    cp -R "$extract/$package/frontend/dist" "$context/frontend"
  fi
done
install -m 0755 "$(dirname "${BASH_SOURCE[0]}")/docker-entrypoint.sh" "$context/docker-entrypoint.sh"
# Intermediate package directories do not belong in the Docker build context.
rm -rf -- "$context/packages"
