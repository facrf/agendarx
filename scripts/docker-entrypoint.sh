#!/bin/sh
set -eu

# Axum's body limit uses usize; a 32-bit executable cannot accept a 5 GiB limit.
if [ "$(getconf LONG_BIT)" = 32 ]; then
  backup_limit="${BACKUP_MAX_UPLOAD_BYTES:-5368709120}"
  case "$backup_limit" in
    ''|*[!0-9]*) ;; # Preserve the application's validation of invalid settings.
    *)
      while [ "${backup_limit#0}" != "$backup_limit" ]; do backup_limit="${backup_limit#0}"; done
      backup_limit="${backup_limit:-0}"
      if [ "${#backup_limit}" -gt 10 ] || [ "$backup_limit" -gt 4294967295 ]; then
        export BACKUP_MAX_UPLOAD_BYTES=4294967295
      fi
      ;;
  esac
fi

exec /usr/local/bin/agendarx "$@"
