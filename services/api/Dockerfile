# 기능: API 이미지 정의.
# 목적: 빌드 도구와 실행 사용자를 나누고, 비밀 파일을 이미지에 넣지 않는다.
# 연관: services/api/src/main.rs의 /actuator/health.
# 로컬 태그 datagrizzly-api:m6 를 빌드했고, 127.0.0.1:5000 레지스트리에 같은 digest로 푸시했다.
# 공개 레지스트리에는 푸시하지 않았다. DB_URL이 없으면 프로세스는 바로 종료한다.
# Go 런타임은 넣지 않는다.

FROM rust:1.92-bookworm AS build
WORKDIR /src
COPY services/api/Cargo.toml services/api/Cargo.lock ./
COPY services/api/src ./src
COPY services/api/migrations ./migrations
ENV CARGO_BUILD_JOBS=2
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends wget \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --create-home --uid 10001 --shell /usr/sbin/nologin app
USER app
WORKDIR /home/app
COPY --from=build /src/target/release/datagrizzly-api /home/app/datagrizzly-api
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD wget -q -O - http://127.0.0.1:8080/actuator/health
CMD ["/home/app/datagrizzly-api"]
