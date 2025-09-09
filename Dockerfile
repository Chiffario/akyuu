FROM rust:1.89.0-alpine3.22 AS builder
RUN apk add build-base

WORKDIR /usr/src/issue-bot

COPY . .

RUN cargo install --path .

FROM alpine:3.22 AS app

COPY --from=builder /usr/local/cargo/bin/issue-bot /usr/local/bin/issue-bot

CMD ["issue-bot", "-c", "/etc/issue-bot/configuration.ron"]