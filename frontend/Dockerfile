# syntax=docker/dockerfile:1
FROM node:22-bookworm AS builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@11.13.0 --activate
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
ENV VITE_OTEL_ENDPOINT=http://localhost:14318
RUN pnpm exec vp build

FROM docker.io/library/caddy:2.9-alpine
COPY --from=builder /app/dist /srv
COPY Caddyfile /etc/caddy/Caddyfile
EXPOSE 5173
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
