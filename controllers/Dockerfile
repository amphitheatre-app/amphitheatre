# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/engine/reference/builder/

################################################################################
# Base image as the foundation for the other build stages in this file.
FROM rust:slim AS chef

# Set the environment variables for the build.
ENV CARGO_UNSTABLE_SPARSE_REGISTRY=true
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN rustup default stable
RUN rustup component add cargo rust-std rustc
RUN cargo install cargo-chef

WORKDIR /app

################################################################################
# Create a stage for cargo chef prepare recipe.
FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

################################################################################
# Create a stage for building/compiling the application.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build our project dependencies, not our application.
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.

COPY . .
RUN cargo build --release --bin amp-controllers

################################################################################
# Create a final stage for running your application.
#
# The following commands copy the output from the "build" stage above and tell
# the container runtime to execute it when the image is run. Ideally this stage
# contains the minimal runtime dependencies for the application as to produce
# the smallest image possible. This often means using a different and smaller
# image than the one used for building the application, but for illustrative
# purposes the "base" image is used here.
FROM gcr.io/distroless/cc:nonroot AS runtime

# Copy the executable from the "building" stage.
COPY --from=builder \
     --chown=nonroot:nonroot \
     /app/target/release/amp-controllers \
     /usr/local/bin/

# What the container should run when it is started
CMD ["/usr/local/bin/amp-controllers"]
