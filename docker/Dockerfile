FROM ubuntu:latest

RUN groupadd -r svanill && useradd --no-log-init -r -g svanill svanill

COPY --chown=svanill:svanill svanill-vault-server /usr/local/bin/
COPY --chown=svanill:svanill svanill-vault-cli /usr/local/bin/

ENV SVANILL_VAULT_DB=/usr/local/share/vault.db

USER svanill

ENTRYPOINT /usr/local/bin/svanill-vault-server
CMD -h