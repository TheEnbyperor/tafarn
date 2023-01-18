FROM rustlang/rust:nightly AS builder
RUN update-ca-certificates
WORKDIR /usr/src/

RUN USER=root cargo new tafarn
WORKDIR /usr/src/tafarn
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
COPY migrations ./migrations
COPY i18n ./i18n
COPY i18n.toml ./i18n.toml
RUN cargo install --path .

FROM debian:buster-slim

RUN apt-get update && apt-get install -y libssl1.1 libpq5 ca-certificates p11-kit-modules \
    && apt-get clean && rm -rf /var/lib/apt/lists/*
RUN update-ca-certificates

WORKDIR /tafarn

COPY --from=builder --chown=0:0 /usr/local/cargo/bin/frontend /tafarn/frontend
COPY --from=builder --chown=0:0 /usr/local/cargo/bin/tasks /tafarn/tasks
COPY --from=builder --chown=0:0 /usr/local/cargo/bin/tafarnctl /tafrarn/tasks
COPY --chown=0:0 static /tafarn/static
COPY --chown=0:0 templates /tafarn/templates

ENTRYPOINT ["/tafarn/frontend"]