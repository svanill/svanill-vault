# svanill-vault-server

An HTTP server to store/retrieve files produced by svanill ([cli](https://github.com/svanill/svanill-cli) or [web](https://github.com/svanill/svanill)) files.

## Build

You need to first generate some files from the openapi specifications.

```
./regenerate_models.sh
```

Then as usual

```
cargo build
```

