# Stage 1: Build
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev postgresql-dev

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && \
    cargo build --release

# Stage 2: Runtime
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache libgcc libpq wget

# Create non-root user
RUN addgroup -g 1000 zid && \
    adduser -D -u 1000 -G zid zid

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/zid /app/zid

# Change ownership
RUN chown -R zid:zid /app

# Switch to non-root user
USER zid

# Expose port
EXPOSE 5555

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:5555/health || exit 1

# Run the application
CMD ["/app/zid"]
