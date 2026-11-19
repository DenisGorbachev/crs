#!/usr/bin/env -S fnox exec --replace -- bash
#MISE raw=true
#MISE tools={"github:dioxuslabs/dioxus"="0.7.10"}

exec dx serve -p crs-web --server --fullstack=false --open=false "$@"
