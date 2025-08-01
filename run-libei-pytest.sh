#!/bin/bash

set -e

if [ "$#" -lt 1 ]; then
	>&2 echo "Usage: run-libei-pytest.sh <path/to/libei> [pytest arguments]"
	exit 1
fi

cargo build --features="calloop" --example reis-demo-server

mkdir -p xdg
export XDG_RUNTIME_DIR=$PWD/xdg
export LIBEI_TEST_SERVER=$PWD/target/debug/examples/reis-demo-server
export LIBEI_TEST_SOCKET=$PWD/xdg/eis-0

# Must already be built
cd "$1/build"
pytest-3 "${@:2}"
