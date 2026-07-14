# アーキテクチャ

## 目的

SigNoz 上で **Traces / Metrics / Logs を相関**し、レイテンシ異常 → Trace → 遅い Span → 同 `trace_id` のログ → SQL / 外部 I/O、という調査を一つの UI で完結できることを検証する。

## 最終構成図

```
                          ┌─ React OTel
                          ├─ Axum OTel traces / metrics / OTLP logs
                          ├─ Caddy JSON logs
                          ├─ PostgreSQL metrics/logs
                          ├─ Garage OTLP traces + /metrics scrape
                          └─ Host CPU/memory/network
                                      │
                                      ▼
                    OpenTelemetry Collector Contrib
                     ├─ OTLP receiver (:4317/:4318 in container,
                     │                 host 14317/14318)
                     ├─ filelog receiver (Caddy / Postgres)
                     ├─ hostmetrics receiver
                     ├─ PostgreSQL receiver
                     ├─ prometheus receiver (Garage only)
                     ├─ processors (memory_limiter, resource, batch)
                     └─ OTLP exporter → SigNoz
                                      │
                                      ▼
                         SigNoz OTel Collector (:4317/:4318)
                                      │
                                      ▼
                                 ClickHouse
                                      │
                                      ▼
                                  SigNoz UI (:8080)
```

## データ経路

### Traces

```
React fetch (+ traceparent)
  → Axum HTTP span（伝播抽出ミドルウェア）
  → Service / handler span
  → SQLx / PostgreSQL span
  → Garage (S3) span  ※Garage 自身も trace_sink → Collector
  → OTLP → Gateway Collector → SigNoz → ClickHouse
```

### Metrics

```
Axum application metrics
hostmetrics
PostgreSQL receiver
Garage admin /metrics（prometheus scrape）
Collector internal metrics
  → Gateway Collector → SigNoz → ClickHouse
```

### Logs

```
Axum OTLP logs（trace_id / span_id 付き）
Caddy access JSON logs
PostgreSQL logs
  → filelog / OTLP → Gateway Collector → SigNoz → ClickHouse
```

## ランタイム分割

| レイヤ | 起動方法 | 役割 |
|--------|----------|------|
| SigNoz | Foundry `casting.yaml`（`just obs-up`） | UI + SigNoz Collector + ClickHouse 等 |
| アプリ基盤 | `infra/compose.yaml`（Podman） | Postgres, Garage, gateway Collector, Caddy, app コンテナ |
| ローカル開発 | ホスト上の `cargo` / `vp` | Backend :3000、Frontend :5173（Vite proxy） |

Gateway Collector はホスト **14317/14318** で受信し、SigNoz の **4317/4318** へ export する（ポート衝突回避）。

## リポジトリ配置

```
signozpoc/
  AGENTS.md                 # エージェント向け要約
  README.md                 # ユーザー向け最短手順
  casting.yaml              # SigNoz Foundry
  justfile
  .env.example
  doc/                      # 本ディレクトリ
  infra/
    compose.yaml
    caddy/Caddyfile
    otel/otel-collector-config.yaml
    garage/garage.toml
    postgres/init.sql
    logs/{caddy,postgres}/  # filelog 用（gitignore 対象の *.log）
  backend/                  # Rust Axum
  frontend/                 # Vite+ React
```

## 主要コンポーネントの対応

| コンポーネント | パス / 設定 |
|----------------|-------------|
| API | `backend/src/` |
| Web | `frontend/src/` |
| マイグレーション | `backend/migrations/` |
| Gateway Collector | `infra/otel/otel-collector-config.yaml` |
| Caddy | `infra/caddy/Caddyfile`（本番相当は :8088） |
| Garage | `infra/garage/garage.toml`（`admin.trace_sink`） |
