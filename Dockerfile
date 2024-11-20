FROM rust:latest AS builder

WORKDIR /app

COPY .cargo .cargo
COPY prisma prisma
COPY prisma-cli prisma-cli
COPY Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release


COPY src src
RUN touch src/main.rs
RUN cargo prisma generate
RUN cargo build --release

RUN strip target/release/commerce_v2

FROM alpine:latest AS release
WORKDIR /app

COPY --from=builder /app/target/release/commerce_v2 .

RUN apk --no-cache add ca-certificates
RUN update-ca-certificates

# Create appuser
ENV USER=commerce_v2_user
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

# Use an unprivileged user.
USER ${USER}:${USER}

ENTRYPOINT ["commerce_v2"]
