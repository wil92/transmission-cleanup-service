FROM rust:1.87.0 AS build-container

# setup dummie projet
RUN USER=root cargo new build_dir
WORKDIR /build_dir

# coping and installing the dependencies
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# coping and build base code
COPY src ./src
RUN cargo build --release

FROM debian:sid-slim

COPY --from=build-container /build_dir/target/release/file_purge .

RUN apt update && apt install sqlite3 -y

CMD ["./file_purge"]
