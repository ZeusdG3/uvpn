# Etapa de compilación
FROM rust:latest AS builder

WORKDIR /usr/src/app

# Copiar todo el proyecto
COPY . .

# Compilar en modo release
RUN cargo build --release


# Imagen final
FROM debian:bookworm-slim

WORKDIR /usr/src/app

# Copiar solo el binario compilado
COPY --from=builder /usr/src/app/target/release/algoritmo_distribuido .

# Comando por defecto
CMD ["./algoritmo_distribuido"]
