FROM rust as builder
WORKDIR /cryptocat
COPY . .
RUN cargo build --release

FROM rust:slim
ARG TELOXIDE_TOKEN
ENV TELOXIDE_TOKEN=${TELOXIDE_TOKEN}
WORKDIR /app
COPY --from=builder /cryptocat/target/release/Cryptocat .
CMD ["/app/Cryptocat"]