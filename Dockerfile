# Stage 1: Builder
FROM rust:alpine3.22 AS builder

RUN apk add --no-cache libressl libressl-dev musl-dev
RUN apk --update --no-cache add libc-dev linux-headers  ca-certificates wget libffi-dev  build-base gcc zlib zlib-dev
RUN apk update && \
    apk add --no-cache protoc


# Set the working directory inside the container
WORKDIR /app

# COPY workspace Cargo.toml
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY target/.rustc_info.json* ./target/
COPY target/CACHEDIR.TAG* ./target/ 
COPY target/release* ./target/release

# Create the `crates` directory
RUN mkdir crates

WORKDIR /app/crates
COPY crates .

# Build the Rust application in release mode
WORKDIR /app
RUN cargo build --all --all-features --all-targets --release


FROM rust:alpine3.22


# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/matchmaking-server ./matchmaking-server
COPY crates/matchmaking/.env ./

# Command to run the application
CMD ["./matchmaking-server"]