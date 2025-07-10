#!/usr/bin/env bash

set -e

if [ "$#" -ne 1 ]; then
	>&2 echo "Usage: update-protocol-bindings.sh <path/to/libei>"
	exit 1
fi

"$1/proto/ei-scanner" \
	--component=eis \
	--output=src/eiproto_eis.rs \
	--jinja-extra-data='{"eis": true}' \
	"$1/proto/protocol.xml" \
	src/eiproto.rs.jinja

"$1/proto/ei-scanner" \
	--component=ei \
	--output=src/eiproto_ei.rs \
	"$1/proto/protocol.xml" \
	src/eiproto.rs.jinja

"$1/proto/ei-scanner" \
	--component=ei \
	--output=src/eiproto_enum.rs \
	"$1/proto/protocol.xml" \
	src/eiproto_enum.rs.jinja

rustfmt src/eiproto_eis.rs
rustfmt src/eiproto_ei.rs
rustfmt src/eiproto_enum.rs
