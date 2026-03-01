# -- Stage 1: Build frontend --------------------------------------------------
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --ignore-scripts
COPY frontend/ .
RUN npm run build

# -- Stage 2: Build Rust binary -----------------------------------------------
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
COPY mcp-server/Cargo.toml mcp-server/Cargo.lock ./mcp-server/
# Create dummy main.rs so cargo can download + cache dependencies
RUN mkdir -p mcp-server/src && echo 'fn main(){}' > mcp-server/src/main.rs
RUN cargo build --release 2>/dev/null || true
# Now copy real source and rebuild
COPY mcp-server/src/ ./mcp-server/src/
RUN touch mcp-server/src/main.rs && cargo build --release

# -- Stage 3: Runtime image ---------------------------------------------------
FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/nosce /usr/local/bin/nosce
COPY --from=frontend /app/mcp-server/static /opt/nosce/static
COPY nosce.yml /opt/nosce/nosce.yml

WORKDIR /opt/nosce

ENV HOST=0.0.0.0
ENV PORT=3000
ENV NOSCE_OUTPUT_DIR=/data

EXPOSE 3000

VOLUME ["/data"]

ENTRYPOINT ["nosce"]
CMD ["serve"]
