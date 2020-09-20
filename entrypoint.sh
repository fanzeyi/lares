#!/bin/sh
set -e

trap 'echo "terminating..." && exit 0' INT TERM

/usr/local/bin/lares "$@" &

wait
