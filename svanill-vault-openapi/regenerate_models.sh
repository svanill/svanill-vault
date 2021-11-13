#!/bin/sh

set -eux

mkdir -p src
rm -fr tmp_build
rm -fr src/models
mkdir tmp_build
mkdir tmp_build/templates
cp openapi3.yml tmp_build/

curl -s https://raw.githubusercontent.com/OpenAPITools/openapi-generator/master/modules/openapi-generator/src/main/resources/rust/model.mustache | grep -v partial_header > tmp_build/templates/model.mustache

podman run -u $(id -u) --rm -v ${PWD}/tmp_build:/local \
    openapitools/openapi-generator-cli \
    generate \
    -g rust \
    -t /local/templates/ \
    --global-property=modelDocs=false \
    -i /local/openapi3.yml \
    -o /local/out/

sed -i 's/crate::models/super/g' tmp_build/out/src/models/*.rs

cp -R tmp_build/out/src/models/ src/models
rm -fr tmp_build
