# Sistema Distribuido en Rust

Este proyecto implementa un sistema distribuido con un **coordinador** y varios **workers** conectados en distnet usando Docker y Docker Compose.
El coordinador asigna tareas (`Task`) a los workers, quienes devuelven resultados (`ResultMsg`).

## Requisitos

- Ubuntu 24.04 (o similar)
- Docker >= 24.x
- Docker Compose v2
- Rust (para desarrollo local, opcional)

## Compilación y construcción

Desde la raíz del proyecto:

```bash
#Construir la imagen del proyecto

docker-compose -f docker/docker-compose.yml build

---

#Levantar los contenedores:

docker-compose -f docker/docker-compose.yml up -d

---

# Verificar que estan corriendo

docker ps

```

## Notas adicionales

- El sistema requiere la conexión del VPN para que los workers puedan comunicarse con el coordinador.
- Si se modifica el código en `rust/`, es necesario reconstruir las imágenes:
