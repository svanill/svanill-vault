#!/bin/sh

# This script adds a default dev user to a local sqlite database file.
# The database file should already exist: is created when you run svanill-server-vault.

if [ $# -ne 1 ] || [ ! -f "$1" ]; then
    echo "Usage: ./add-dev-user.sh <db name e.g. vault.db>"
fi

echo Add user to local.db ...
# We are going to insert (username, challenge, answer) into `user` table.
# As `challenge` use a random string (well, not so much here), encrypted by svanill using password 'x'
# e.g. svanill -i <(echo 9E3245D722A884F02A5DE6030A904C9C) -p x enc
sqlite3 local.db <<< "INSERT OR IGNORE INTO user VALUES ('local-user','00000186a09186e5f1a5b2cb6f4961c9126219566ce5a53a91bf81df4e5b65c81377269f99cb84e77abbe3fff0ba3fb164083e6d69e80d169480f5901df246c441c5b741823c173b2fdd6bf739a5079c36d7','9E3245D722A884F02A5DE6030A904C9C');"
echo Done