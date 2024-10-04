#!/usr/bin/env bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

docker run -i --rm \
  --volume=$DIR/:"/usr/app" \
  -u $(id -u ${USER}):$(id -g ${USER}) \
  ggrossetie/asciidoctor-web-pdf:latest \
  --template-require ./template.js resume.adoc

