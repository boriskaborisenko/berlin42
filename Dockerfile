# Production Dockerfile for the future Render deployment.
# Local development should run the Rust server directly from ./backend.

FROM rust:1-bookworm AS backend-builder
WORKDIR /app

COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
COPY backend/src ./backend/src

WORKDIR /app/backend
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/backend/target/release/skill-creator-server /app/skill-creator-server
COPY config_models.json /app/config_models.json
COPY prompts/master /app/prompts/master

ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
ENV CONFIG_MODELS_PATH=/app/config_models.json
ENV PROMPTS_DIR=/app/prompts/master

EXPOSE 8080

CMD ["/app/skill-creator-server"]
