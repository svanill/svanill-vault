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

## FAQ

- why don't you use the generated client and server too?

When I tried last time the server failed to generate, and the client was a bit in a flux because of async support.

Will try again when openapi-generator is more mature.
