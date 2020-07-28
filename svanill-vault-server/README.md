# svanill-vault-server

An HTTP server to store/retrieve files produced by svanill ([cli](https://github.com/svanill/svanill-cli) or [web](https://github.com/svanill/svanill)) files.

## Build


```
cargo build
```

## Openapi changes

If you change `openapi3.yml` you will have to rebuild and commit the model files

```
./regenerate_models.sh
git add openapi3.yml
git add src/openapi_models
git commit -m "Update openapi models"
```
