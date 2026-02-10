# Build stage
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
ARG PACKAGE
RUN cargo build --release -p ${PACKAGE}

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
ARG PACKAGE
COPY --from=builder /app/target/release/${PACKAGE} /usr/local/bin/app
# Copy docs for rwire-docs (no-op for other packages, mkdir -p is safe)
COPY --from=builder /app/apps/rwire-docs/docs/ /app/docs/
ENV DOCS_DIR=/app/docs
EXPOSE 9000
CMD ["app"]
