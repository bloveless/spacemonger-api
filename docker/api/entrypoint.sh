#!/bin/bash
set -e

if [ "$1" = "watch" ]; then
  shift

  exec cargo watch -w /app/api/src -w /app/spacemonger -x "run -- " "$@"
fi

if [ "$1" = "shell" ]; then
  exec /bin/bash
fi

exec /app/spacemonger-api "$@"
