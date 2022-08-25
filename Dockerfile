FROM node:16 as frontendbuilder

COPY ./frontend /frontend

RUN cd /frontend && rm -rf dist && npm i && npm run build

FROM eclipse-temurin:17.0.1_12-jdk as backendbuilder

COPY ./backend/*.gradle ./backend/gradle.* ./backend/gradlew /build/backend/
COPY ./backend/gradle /build/backend/gradle
WORKDIR /build/backend
RUN chmod +x gradlew && ./gradlew --version

COPY ./backend /build/backend
COPY --from=frontendbuilder /frontend/dist /build/frontend/dist

RUN cd /build/backend && ./gradlew bootJar

FROM debian:bullseye-slim as baseimage

ENV LANG='en_US.UTF-8' LANGUAGE='en_US:en' LC_ALL='en_US.UTF-8'
ENV PATH=/opt/java/openjdk/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV JAVA_HOME=/opt/java/openjdk

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive \
    && apt-get update     && DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends tzdata curl wget ca-certificates fontconfig locales binutils     && echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen \
    && locale-gen en_US.UTF-8 \
    && wget -O /tmp/openjdk.tar.gz https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.4.1%2B1/OpenJDK17U-jre_x64_linux_hotspot_17.0.4.1_1.tar.gz \
    && echo "e96814ee145a599397d91e16831d2dddc3c6b8e8517a8527e28e727649aaa2d1 */tmp/openjdk.tar.gz" | sha256sum -c - && mkdir -p "$JAVA_HOME" \
    && tar --extract --file /tmp/openjdk.tar.gz --directory "$JAVA_HOME" --strip-components 1 --no-same-owner && rm /tmp/openjdk.tar.gz \
    && find "$JAVA_HOME/lib" -name '*.so' -exec dirname '{}' ';' | sort -u > /etc/ld.so.conf.d/docker-openjdk.conf && ldconfig &&  java -Xshare:dump \
    && apt-get install -y --no-install-recommends chromium-l10n \
      fonts-liberation \foundEntity
      fonts-roboto \
      hicolor-icon-theme \
      libcanberra-gtk-module \
      libexif-dev \
      libgl1-mesa-dri \
      libgl1-mesa-glx \
      libpangox-1.0-0 \
      libv4l-0 \
      fonts-symbola \
      chromium-driver \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN groupadd klubu -g 1000 && useradd klubu -m -u 1000 -g 1000 -G audio,video && mkdir /chromedata && chown klubu:klubu /chromedata

USER klubu

FROM baseimage

WORKDIR /app

COPY --chown=1000:1000 --from=backendbuilder /build/backend/build/libs/klubu-*.jar /app/klubu.jar

ENTRYPOINT ["java", "-jar", "klubu.jar"]
