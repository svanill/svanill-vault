# svanill-vault-openapi

Openapi definition and materialized rust models.

## Build

```
cargo build
```

## Openapi changes

If you edit `openapi3.yml` you will have to rebuild and commit the model files

```
./regenerate_models.sh
git add openapi3.yml
git add src/models
git commit -m ...
```
