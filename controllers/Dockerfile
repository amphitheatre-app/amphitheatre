# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/engine/reference/builder/

################################################################################
# Pick a base image to serve as the foundation for the other build stages in
# this file.
#
# Using the `rust-musl-builder` as base image, instead of
# the official Rust toolchain. (see https://github.com/clux/muslrust)
FROM clux/muslrust:nightly-2022-12-01 AS chef

WORKDIR /app

# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef

################################################################################
# Create a stage for cargo chef prepare recipe.
FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

################################################################################
# Create a stage for building/compiling the application.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
# Notice that we are specifying the --target flag!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .

# Build application
RUN cargo build --release --target x86_64-unknown-linux-musl --bin amp-controllers
RUN strip -s /app/target/x86_64-unknown-linux-musl/release/amp-controllers

################################################################################
# Create a final stage for running your application.
#
# The following commands copy the output from the "build" stage above and tell
# the container runtime to execute it when the image is run. Ideally this stage
# contains the minimal runtime dependencies for the application as to produce
# the smallest image possible. This often means using a different and smaller
# image than the one used for building the application, but for illustrative
# purposes the "base" image is used here.
FROM gcr.io/distroless/static:nonroot AS runtime

# Copy the executable from the "building" stage.
COPY --from=builder \
     --chown=nonroot:nonroot \
     /app/target/x86_64-unknown-linux-musl/release/amp-controllers \
     /usr/local/bin/

# What the container should run when it is started
ENTRYPOINT ["/usr/local/bin/amp-controllers"]
