#!/bin/bash
set -e

if [ "$1" = "shell" ]; then
  exec /bin/bash
fi

exec /app/spacemongerd "$@"
