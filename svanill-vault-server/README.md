# svanill-vault-server

An HTTP server to store/retrieve files produced by svanill ([cli](https://github.com/svanill/svanill-cli) or [web](https://github.com/svanill/svanill)).

An authenticated user can push, list or remove files to a dedicated S3 bucket.

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
If the database file does not exist, it will be created and a migration will run automatically.
You can add users by running a query such as

```
sqlite3 test.db
sqlite> .schema
CREATE TABLE __diesel_schema_migrations (version VARCHAR(50) PRIMARY KEY NOT NULL,run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE user (
  username VARCHAR(50) NOT NULL PRIMARY KEY,
  challenge VARCHAR(255) NOT NULL,
  answer VARCHAR(32) NOT NULL
);
sqlite> INSERT INTO user VALUES ('your username', 'the challenge', 'the answer');
```

## Authentication

Users authenticate by requesting a challenge and then providing the answer to that challenge.

`answer` should be a random string, e.g. generated with `hexdump -n 16 -e '4/4 "%08X" 1 "\n"' /dev/random`

`challenge` is the answer encrypted with a symmetric algorithm (supposedly using Svanill ([web](https://github.com/svanill/svanill) or [cli](https://github.com/svanill/svanill-cli)).

It works this way so that a Svanill user can use a single password to both encrypt/decrypt files and login securely (Svanill encrypt using AES-GCM which doesn't suffer from known-plaintext attack).

