# -- Stage 1: Build web UI ----------------------------------------------------
FROM node:22-alpine AS webui
WORKDIR /app/webui
COPY webui/package.json webui/package-lock.json* ./
RUN npm ci --ignore-scripts
COPY webui/ .
RUN npm run build

# -- Stage 2: Build Rust binary -----------------------------------------------
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
COPY cli/Cargo.toml cli/Cargo.lock ./cli/
# Create dummy main.rs so cargo can download + cache dependencies
RUN mkdir -p cli/src && echo 'fn main(){}' > cli/src/main.rs
RUN cargo build --release 2>/dev/null || true
# Now copy real source and rebuild
COPY cli/src/ ./cli/src/
RUN touch cli/src/main.rs && cargo build --release

# -- Stage 3: Runtime image ---------------------------------------------------
FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/nosce /usr/local/bin/nosce
COPY --from=webui /app/cli/static /opt/nosce/static
COPY nosce.config.yml /opt/nosce/nosce.config.yml

WORKDIR /opt/nosce

ENV HOST=0.0.0.0
ENV PORT=3000
ENV NOSCE_OUTPUT_DIR=/data

EXPOSE 3000

VOLUME ["/data"]

ENTRYPOINT ["nosce"]
CMD ["serve"]
