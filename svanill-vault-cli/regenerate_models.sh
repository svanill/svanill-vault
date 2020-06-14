#!/bin/sh

set -eux

rm -fr tmp_build
rm -fr src/models
mkdir tmp_build
cp openapi3.yml tmp_build/

docker run -u $(id -u) --rm -v ${PWD}/tmp_build:/local \
    openapitools/openapi-generator-cli \
    generate \
    -g rust \
    --global-property=models \
    --global-property=modelDocs=false \
    -i /local/openapi3.yml \
    -o /local/out/

cp -R tmp_build/out/src/models/ src/models
rm -fr tmp_build
