FROM rust:1.99-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY . .

RUN cargo build --release --locked -p danfse-server

FROM alpine:latest

RUN apk update && apk add --no-cache \
    ca-certificates \
    tzdata

ENV TZ=America/Sao_Paulo

WORKDIR /app

COPY --from=builder /app/target/release/danfse-server ./danfse-service

EXPOSE 8080

ENTRYPOINT ["./danfse-service"]
