#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-all}"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"

echo "==> Pushing branch '$BRANCH' to GitHub..."
git push origin "$BRANCH"

echo "==> Triggering GitHub Actions macOS build (target: $TARGET)..."
gh workflow run macos-build.yml --ref "$BRANCH" -f "build_target=$TARGET"

echo "==> Waiting for workflow run to register..."
sleep 4
RUN_ID="$(gh run list --workflow=macos-build.yml --branch="$BRANCH" --limit 1 --json databaseId -q '.[0].databaseId')"
echo "==> Watching run #$RUN_ID..."
gh run watch "$RUN_ID" --exit-status

echo "==> Downloading binaries into ./dist/..."
rm -rf dist
mkdir -p dist
gh run download "$RUN_ID" --name nightfall-macos-arm64 --dir dist
chmod +x dist/*
ls -lh dist/
echo "==> Done! Run ./dist/nightfall-editor or ./dist/nightfall-game"
