# オブザーバビリティ

## サービス名

| service.name | 出所 |
|--------------|------|
| `equipment-reservation-web` | React OTel（`frontend/src/otel.ts`） |
| `equipment-reservation-api` | Axum（`OTEL_SERVICE_NAME`） |
| `garage` | Garage `trace_sink` |
| `caddy` / `postgres` | filelog の resource 付与 |

## エンドポイント

| 経路 | 値 |
|------|-----|
| App → Gateway Collector | `OTEL_EXPORTER_OTLP_ENDPOINT` 既定 `http://localhost:14317`（gRPC） |
| Browser → Gateway | `VITE_OTEL_ENDPOINT` 既定 `http://localhost:14318`（HTTP OTLP `/v1/traces`） |
| Gateway → SigNoz | `SIGNOZ_OTLP_ENDPOINT` 既定 `host.containers.internal:4317`（スキームなし） |
| SigNoz UI | `http://localhost:8080` |

## Collector 設定

ファイル: `infra/otel/otel-collector-config.yaml`

| 信号 | receivers |
|------|-----------|
| traces | `otlp`（Garage `trace_sink` 含む） |
| metrics | `otlp`, `hostmetrics`, `postgresql`, `prometheus`(Garage:3903) |
| logs | `otlp`, `filelog/caddy`, `filelog/postgres` |

**入れない**: Prometheus サーバ、Grafana Alloy、Tempo、Loki。

## アプリ実装メモ

- HTTP 入来: `backend/src/api/otel_middleware.rs` が `traceparent` を抽出して親コンテキストを設定
- handler / DB / Garage: `#[instrument]` および TraceLayer
- Backend の `tracing` event は OTLP Logs として gateway Collector へ直接送信され、アクティブ span の `trace_id` / `span_id` を LogRecord に設定する。標準出力の JSON はローカル診断用。
- Caddy は `tracing` directive で access log に `traceID` / `spanID` を出力し、Collector の `trace_parser` が LogRecord の相関フィールドへ設定する。
- Collector 未起動時: backend はテレメトリ初期化失敗を警告して継続起動可能

## 調査フロー（SigNoz UI）

意図した検証ストーリー:

```
レイテンシ異常
  → 該当 Trace
  → 遅い Span（例: slow_probe / pg_sleep / Garage put）
  → 同じ trace_id の Logs
  → SQL やオブジェクトストレージの原因確認
```

手順:

1. アプリでログインし予約する、または「遅いクエリを実行」
2. SigNoz → Traces で web / api サービスをフィルタ
3. 遅い Span を開く
4. 同一 Trace の Logs で Axum（OTLP Logs）/ Caddy（filelog）を確認
5. Metrics で host / PostgreSQL / Garage scrape を確認

## デモ用エンドポイント

`GET /api/demo/slow` — Postgres `pg_sleep(1.5)`。SigNoz で遅い SQL span を出すため。

## Postgres ログ方針

Compose の Postgres は `log_statement=mod`（変更系のみ）。`all` は使わず、認証まわりの SELECT リテラルが観測パイプラインへ過剰流入しないようにする。
