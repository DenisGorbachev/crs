#!/usr/bin/env -S fnox exec --replace -- bash
#MISE raw=true

exec dx serve -p crs-web --server --fullstack=false --open=false "$@"
