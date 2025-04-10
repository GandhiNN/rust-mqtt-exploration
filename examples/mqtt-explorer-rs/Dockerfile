# Use a Rust base image with Cargo installed
# FROM rust:latest AS builder
FROM jscharnitzke/rust-build-windows:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# # Copy the Cargo.toml and Cargo.lock files
# COPY Cargo.toml Cargo.lock ./

# # Copy the source code
# COPY ./src ./src

# Update base image and install mingw binaries
# To build Windows applications
# RUN apt update
# RUN apt upgrade -y
# RUN apt install -y g++-mingw-w64-x86-64

# RUN rustup target add x86_64-pc-windows-gnu
# RUN rustup toolchain install stable-x86_64-pc-windows-gnu

# Build the image
CMD ["cargo", "build", "--target", "x86_64-pc-windows-gnu"]
