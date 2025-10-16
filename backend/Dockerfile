FROM rust:1.74 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/expense_portal /usr/local/bin/expense_portal
COPY migrations ./migrations
EXPOSE 8080
ENV RUST_LOG=info
CMD ["expense_portal"]
