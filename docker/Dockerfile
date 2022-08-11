FROM ubuntu:latest

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

RUN groupadd -r svanill && useradd --no-log-init -r -g svanill svanill

COPY --chown=svanill:svanill svanill-vault-server /usr/local/bin/
COPY --chown=svanill:svanill svanill-vault-cli /usr/local/bin/
RUN chmod +x /usr/local/bin/svanill-vault-server
RUN chmod +x /usr/local/bin/svanill-vault-cli

ENV SVANILL_VAULT_DB=/usr/local/share/vault.db

USER svanill

ENTRYPOINT /usr/local/bin/svanill-vault-server
CMD -h