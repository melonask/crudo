FROM rust:1.97-alpine3.24 AS build
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config/minimal.toml ./config/minimal.toml

FROM build AS builder
RUN cargo build --locked --release && mkdir /data

FROM scratch AS runtime
COPY --from=builder /app/target/release/crudo /usr/local/bin/crudo
COPY config/minimal.toml /etc/crudo/config.toml
COPY --from=builder --chown=10001:10001 /data /data
USER 10001:10001
WORKDIR /data
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/crudo"]
CMD ["--config", "/etc/crudo/config.toml", "--address", "0.0.0.0:3000"]

FROM build AS e2e-builder
COPY tests ./tests
COPY config ./config
RUN mkdir /out \
    && CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
       cargo test --locked --release --test e2e --no-run \
    && for executable in target/release/deps/e2e-*; do \
         if [ -x "$executable" ]; then cp "$executable" /out/e2e; break; fi; \
       done \
    && test -x /out/e2e

FROM alpine:3.24 AS test
COPY --from=e2e-builder /out/e2e /usr/local/bin/e2e
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/e2e"]
CMD ["--ignored", "--nocapture"]
