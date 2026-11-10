
#!/usr/bin/env bash
set -euo pipefail

VERSION="1.0.0"
NAME="resting-chair"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/$NAME-$VERSION"

git archive HEAD | tar -x -C "$TMP/$NAME-$VERSION"

cd "$TMP/$NAME-$VERSION/src-tauri"

cargo vendor vendor > .cargo-config.tmp
mkdir -p .cargo
mv .cargo-config.tmp .cargo/config.toml

cd "$TMP/$NAME-$VERSION"

tar czf "$OLDPWD/../${NAME}_${VERSION}.orig.tar.gz" \
    --exclude='.git' \
    .
