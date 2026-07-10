FROM rust:1.93-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --version 0.16.0 --locked

WORKDIR /build
COPY . /build

RUN cd frontend && trunk build --release
RUN cargo build --release --package backend

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates fontconfig poppler-utils tesseract-ocr tesseract-ocr-deu tesseract-ocr-eng \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 klubu \
    && useradd --uid 1000 --gid 1000 --create-home --shell /usr/sbin/nologin klubu \
    && mkdir -p /app/config /app/frontend/dist /app/templates /app/document_storage /app/mail_storage \
    && chown -R klubu:klubu /app

WORKDIR /app

COPY --from=builder /build/target/release/backend /app/backend
COPY --from=builder /build/frontend/dist /app/frontend/dist
COPY --from=builder /build/templates /app/templates

ENV KLUBU_EXPORT_TEMPLATES_PATH=/app/templates \
    KLUBU_DOCUMENT_STORAGE_PATH=/app/document_storage \
    KLUBU_MAIL_STORAGE_PATH=/app/mail_storage

USER klubu
EXPOSE 8080 2525 2143
ENTRYPOINT ["/app/backend"]
