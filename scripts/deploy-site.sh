#!/bin/bash
# Deploys site/ to the server. Stylesheet, script and favicon URLs get a content hash (?v=...) so browsers and the CDN
# never keep an old copy next to new HTML. Usage: scripts/deploy-site.sh [user@host]
set -euo pipefail
cd "$(dirname "$0")/.."
HOST="${1:-debian@92.222.80.118}"
for f in site/index.html site/fr/index.html site/404.html; do
  for asset in style.css app.js favicon.svg; do
    [ -f "site/$asset" ] || continue
    hash=$(md5sum "site/$asset" | cut -c1-10)
    sed -i -E "s#(/${asset})(\?v=[0-9a-f]+)?\"#\1?v=${hash}\"#g" "$f"
  done
done
rsync -a --delete site/ "$HOST:/var/www/slimy/"
echo "deployed to $HOST"
