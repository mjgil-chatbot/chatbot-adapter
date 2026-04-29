#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="agent-adapter-builder"
IMAGE_TAG="latest"

docker build -t "$IMAGE_NAME:$IMAGE_TAG" .
