FROM rustlang/rust:nightly as builder
ENV RUSTUP_TOOLCHAIN=nightly
ENV CARGO_TERM_COLOR=always
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Pre-fetch and compile dependencies by building a minimal binary. This allows
# later Docker builds to reuse the cached dependency layer when only source
# files change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && printf 'pub fn placeholder() {}\\n' > src/lib.rs \
    && printf 'fn main() {}\\n' > src/main.rs \
    && printf 'fn main() {}\\n' > src/bin/migrator.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/expense_portal /usr/local/bin/expense_portal
COPY migrations ./migrations
EXPOSE 8080
ENV RUST_LOG=info
CMD ["expense_portal"]
