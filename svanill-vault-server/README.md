# svanill-vault-server

An HTTP server meant to be used to store/retrieve files produced by svanill ([cli](https://github.com/svanill/svanill-cli) or [web](https://github.com/svanill/svanill)).

Authenticated users can manage their own files, stored in a S3 bucket.


## Authentication

When creating an account, users must provide a pair `answer` / `challenge`. They later authenticate by requesting their challenge and then providing the answer to that challenge.

`answer` should be a random string of non-trivial length, e.g. the output of `hexdump -n 16 -e '4/4 "%08X" 1 "\n"' /dev/random`

`challenge` is the answer encrypted with a symmetric algorithm (supposedly using Svanill ([web](https://github.com/svanill/svanill) or [cli](https://github.com/svanill/svanill-cli)).

It works this way so that a Svanill user can use a single password to both encrypt/decrypt files and login securely (Svanill encrypt using AES-GCM which doesn't suffer from known-plaintext attack).


## Third party services

Required:
- an S3 compatible service (AWS S3, minio, ...)

Optional:
- Sentry (to log errors)

Currently users data is read from a SQLite db, so no external db is required.

## Build

You will need [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), the Rust package manager.

```
cargo build
```

## Run

The build artifacts end in the target folder at the root of the project (as usual for multicrate Rust repositories).

AWS credential by default are read by env variables `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`, but there are other fallbacks, see [rusoto doc](https://github.com/rusoto/rusoto/blob/master/AWS-CREDENTIALS.md).
You can also pass them as args (see `--help`).

```
cargo run # or execute the binary from `../target/debug` or `../target/release`
```

```
# most params can be provided as env variables, here we just use arguments
RUST_LOG=trace,actix_server=trace,actix_web=trace cargo run -- \
    --s3-access-key-id=test_s3_access_key \
    --s3-secret-access-key=test_s3_secret_key \
    --s3-bucket testbucket \
    --s3-region=us-east-1 \
    --s3-endpoint=http://localhost:9000 \
    -H 127.0.0.1 \
    -P 5000 \
    -d test.db \
    -v
```

## Database

svanill-vault-server access a read only SQLite database file.
Upon first run, if the database file does not exist, it will be created and migrations will run automatically.

To add a user you must first generate the pair `answer` / `challenge`.

```
# generate a random string, such as D831E09A209E23261FBD2DC5E3321112
ANSWER=$(hexdump -n 16 -e '4/4 "%08X" 1 "\n"' /dev/random)
echo "Answer is: $ANSWER"
svanill -i <(echo $ANSWER) -p "the password" enc # note, better not use -p
# The challenge is now the output of svanill-cli
```

Then you can add them to the database.

```
$ sqlite3 test.db
sqlite> -- display the database schema
sqlite> .schema
CREATE TABLE __diesel_schema_migrations (version VARCHAR(50) PRIMARY KEY NOT NULL,run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE user (
  username VARCHAR(50) NOT NULL PRIMARY KEY,
  challenge VARCHAR(255) NOT NULL,
  answer VARCHAR(32) NOT NULL
);
sqlite> -- add a new user
sqlite> INSERT INTO user VALUES ('your username', 'the challenge', 'the answer');
```

To have svanill-vault-cli later authenticate correctly, you are expected to produce the challenge by encrypting the answer using svanill-cli

## Local development

To try svanill-vault-server, you need to
- have an S3 like server ready (or start one such as `minio` locally)
- start svanill-vault-server
- connect to it, either directly (e.g. using `curl`) or through `svanill-vault-cli`


### Start a local S3 compatible server


```
podman run -p 9000:9000 -p 9001:9001 -e "MINIO_ROOT_USER=svanill-vault-local-access-key" -e "MINIO_ROOT_PASSWORD=svanill-vault-s3-secret" quay.io/minio/minio server /data --console-address ":9001"
```

Minio api can be reached at port 9000, admin console at port 9001.
**IMPORTANT: Create a bucket** using the admin console at `http://127.0.0.1:9001`, let's call it `svanill-local-bucket`.

### Start svanill-vault-server


```
RUST_LOG=trace,actix_server=trace,actix_web=trace cargo run -- \
  --s3-access-key-id=svanill-vault-local-access-key \
  --s3-secret-access-key=svanill-local-s3-secret \
  --s3-bucket svanill-local-bucket \
  --s3-region=us-east-1 \
  --s3-endpoint=http://localhost:9000 \
  -H 127.0.0.1 \
  -P 5000 \
  -d local.db \
  -v
```

Add a default dev user, it will have these properties:
- username `local-user`
- answer `9E3245D722A884F02A5DE6030A904C9C`
- the challenge is generated using password `x` with `svanill` (to simplify usage of `svanill-vault-cli`).

```
./add-dev-user.sh local.db
```

### Try to connect with svanill-vault-cli

```
cargo run svanill-vault-cli -- \
  -h localhost:5000 \
  -u local-user \
  -a 9E3245D722A884F02A5DE6030A904C9C \
  ls
```

Remember that you can configure default host/user/answer at
`~/.config/svanill-vault-cli/svanill-vault-cli.toml` (or similar, depending on your OS).
so that you could simply run

```
cargo run svanill-vault-cli -- ls
```


