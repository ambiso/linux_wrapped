FROM rust
COPY . /app
WORKDIR /app
RUN cargo install --path .
CMD linux_wrapped