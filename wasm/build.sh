#!/usr/bin/env bash
# compila o wasm e gera o glue js em bin/
set -euo pipefail

raiz="$(cd "$(dirname "$0")/.." && pwd)"
saida="${1:-$raiz/bin}"
alvo=wasm32-unknown-unknown

cd "$raiz"
cargo build -p danfse-wasm --target "$alvo" --release
wasm-bindgen --target web --out-dir "$saida" --out-name danfse \
  "target/$alvo/release/danfse_wasm.wasm"

wasm="$saida/danfse_bg.wasm"
js="$saida/danfse.js"
for f in "$wasm" "$js"; do
  if ! test -s "$f"; then
    echo "ERROR: $f is missing or empty" >&2
    exit 1
  fi
done
echo "WASM build OK: $wasm ($(wc -c <"$wasm" | tr -d ' ') bytes), $js ($(wc -c <"$js" | tr -d ' ') bytes)"
