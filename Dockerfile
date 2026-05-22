##### Build echelon-webui

FROM rust:trixie AS chef
WORKDIR /app

# Install cargo-chef
RUN cargo install cargo-chef

# Install Dioxus CLI
RUN curl -sSL https://dioxus.dev/install.sh | bash

# Install Emscripten EMSDK
COPY scripts/install-emsdk.sh scripts/install-emsdk.sh
RUN bash scripts/install-emsdk.sh

# Planner stage to analyze dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage to compile the application
FROM chef AS builder
ARG CARGO_BUILD_JOBS=8
ENV CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS

# Cache dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code last (most likely to change)
COPY . .

# Build ocgcore
RUN git submodule update --init --recursive
RUN scripts/build_ocgcore.sh

# Build the web app
RUN dx bundle --platform web --debug-symbols=false --out-dir bundle

# Runtime stage - serve with nginx
FROM nginx:alpine AS runtime

# Copy built assets to nginx
COPY --from=builder /app/bundle/public /usr/share/nginx/html

# Copy custom nginx config
COPY --from=builder /app/config/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]