FROM chromedp/headless-shell:97.0.4692.99 AS headless-shell

FROM node:16 as frontendbuilder

COPY ./frontend /frontend

RUN cd /frontend && rm -rf dist && npm i && npm run build

FROM eclipse-temurin:17.0.1_12-jdk as backendbuilder

COPY /backend. /build/backend
COPY --from=frontendbuilder /frontend/dist /build/frontend/dist

RUN cd backend && chmod +x gradlew && ./gradlew bootJar

FROM debian:bullseye-slim

ENV LANG='en_US.UTF-8' LANGUAGE='en_US:en' LC_ALL='en_US.UTF-8'

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends tzdata curl \
                                      ca-certificates fontconfig locales \
                                      python-is-python3 binutils \
                                      libnspr4 libnss3 libexpat1 libfontconfig1 libuuid1 \
    && echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen \
    && locale-gen en_US.UTF-8 \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ENV JAVA_VERSION jdk-17.0.2+8


RUN set -eux; \
    ARCH="$(dpkg --print-architecture)"; \
    case "${ARCH}" in \
       aarch64|arm64) \
         ESUM='6ef7a28d0d844fe347ab18f65a91db744547321fe8a101d883bd80722183ab64'; \
         BINARY_URL='https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.2%2B8/OpenJDK17U-jre_aarch64_linux_hotspot_17.0.2_8.tar.gz'; \
         ;; \
       amd64|i386:x86-64) \
         ESUM='292ed702d95f5690e52e171afe9f3050b9d2fb803456b155c831735fad0f17c0'; \
         BINARY_URL='https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.2%2B8/OpenJDK17U-jre_x64_linux_hotspot_17.0.2_8.tar.gz'; \
         ;; \
       *) \
         echo "Unsupported arch: ${ARCH}"; \
         exit 1; \
         ;; \
    esac; \
    curl -LfsSo /tmp/openjdk.tar.gz ${BINARY_URL}; \
    echo "${ESUM} */tmp/openjdk.tar.gz" | sha256sum -c -; \
    mkdir -p /opt/java/openjdk; \
    cd /opt/java/openjdk; \
    tar -xf /tmp/openjdk.tar.gz --strip-components=1; \
    rm -rf /tmp/openjdk.tar.gz;

ENV JAVA_HOME=/opt/java/openjdk \
    PATH="/opt/java/openjdk/bin:$PATH"

COPY --from=headless-shell /headless-shell /headless-shell
COPY --from=backendbuilder 
