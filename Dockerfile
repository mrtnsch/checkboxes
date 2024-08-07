# Use the official Rust image as the base image
FROM rust:1.70 AS builder

WORKDIR /app
# Copy cargo files to container
COPY Cargo.toml Cargo.lock ./

# Create fake main.rs file in src and build
RUN mkdir ./src && echo 'fn main() { println!("Dummy!"); }' > ./src/main.rs
RUN cargo build --release

# Copy real src files over
RUN rm -rf ./src
COPY ./src ./src

# The last modified attribute of main.rs needs to be updated manually,
# otherwise cargo won't rebuild it.
RUN touch -a -m ./src/main.rs

RUN cargo build --release

# Create a new minimal image
FROM debian:bullseye-slim

ARG APP=/usr/src/app
ENV APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /app/target/release/checkboxes ${APP}/checkboxes
COPY ./static ${APP}/static

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

# Expose the port(s) that your application listens on
EXPOSE 3030

# Run the application
CMD ["./checkboxes"]