FROM --platform=linux/amd64 clux/muslrust:stable AS builder
RUN \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/build \
    --mount=type=bind,source=.,target=/volume \
    cargo build --release --target-dir /build && \
    cp /build/x86_64-unknown-linux-musl/release/bottle_server /bottle_server

FROM scratch
COPY --from=builder /bottle_server /app/bottle_server
WORKDIR /app
CMD ["./bottle_server"]