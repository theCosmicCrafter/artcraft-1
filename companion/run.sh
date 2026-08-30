#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
export PYTHONUTF8=1
export PYTHONPATH="$DIR:$DIR/backend${PYTHONPATH:+:$PYTHONPATH}"
export ARTCRAFT_STUDIO_DISTRIBUTION="${ARTCRAFT_STUDIO_DISTRIBUTION:-official}"
exec "$DIR/python/bin/python3" "$DIR/backend/main.py" "$@"
