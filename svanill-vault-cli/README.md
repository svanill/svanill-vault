# svanill-vault-cli

A command line client for svanill-vault

## Example usage

```
> echo "this is just a test" > some_file
> svanill-vault-cli push some_file
> svanill-vault-cli ls
> rm some_file
> svanill-vault-cli pull some_file
> cat some_file
> svanill-vault-cli rm some_file
> svanill-vault-cli ls
> cat some_file
```

You can change destination filenames with command line options.
Defaults have been choosen to reduce command line options in day to day use and may differ from some classic conventions.

## Build

You need to first generate some files from the openapi specifications.

```
./regenerate_models.sh
```

Then as usual

```
cargo build
```
