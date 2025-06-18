FROM ubuntu:22.04 AS base

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
ADD . dyndns
WORKDIR /dyndns
RUN cargo build --release
RUN mv target/release /app
WORKDIR /app
EXPOSE 8079/tcp
ENTRYPOINT [ "./dyndns" ]