#!/usr/bin/env bash
# Dedicated disposable local endpoint; never use production credentials or data.
set -euo pipefail
mode="${1:?usage: endpoint-fixture.sh install|serve|bucket WORKDIR}"
root="${2:?explicit fixture work directory required}"
url=https://github.com/seaweedfs/seaweedfs/releases/download/4.53/linux_amd64.tar.gz
sha=d1a7dec9f48d8b33895a0ea8951971defc2dc5312906dcc76fa5a5f74a6ee90d
case "$mode" in
  install)
    mkdir -p "$root/bin" "$root/data"
    curl --fail --location --max-time 120 "$url" --output "$root/seaweedfs-4.53.tar.gz"
    printf '%s  %s\n' "$sha" "$root/seaweedfs-4.53.tar.gz" | sha256sum --check --strict
    tar -xzf "$root/seaweedfs-4.53.tar.gz" -C "$root/bin" weed
    ;;
  serve)
    exec "$root/bin/weed" server -dir="$root/data" -s3 -s3.port=28433 \
      -master.port=29351 -master.port.grpc=39352 -volume.port=28481 -volume.port.grpc=38482 \
      -filer -filer.port=28891 -filer.port.grpc=38892 -ip=127.0.0.1 -ip.bind=127.0.0.1 -master.telemetry=false
    ;;
  bucket)
    for attempt in {1..60}; do
      if curl --fail --silent --max-time 1 -X PUT http://127.0.0.1:28433/molten-live; then exit 0; fi
      sleep 1
    done
    echo 'local endpoint did not become ready' >&2
    exit 1
    ;;
  *) echo 'unknown fixture mode' >&2; exit 2 ;;
esac
