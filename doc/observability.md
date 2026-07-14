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
| Gateway → SigNoz | `SIGNOZ_GATEWAY_OTLP_ENDPOINT` 既定 `signoz-ingester:4317`（Foundry の `signoz-network` 内） |
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

## リポジトリ管理ダッシュボード

定義は [`infra/signoz/dashboards/equipment-reservation-observability.json`](../infra/signoz/dashboards/equipment-reservation-observability.json) に固定する。SigNoz UI で Editor 権限のサービスアカウント API キーを作成して `.env` の `SIGNOZ_API_KEY` に設定した後、以下を実行する。

```bash
just dashboard-sync
```

`dashboard-provisioner` は one-shot コンテナであり、JSON 定義と同期スクリプトを `:ro` でマウントする。同じ内部名のダッシュボードがあれば更新、なければ作成する。定義には次の 3 セクションを含める。

- **API Golden Signals**: route 別 request rate、4xx/5xx、p95 latency、slow probe、status、trace volume
- **Trace / Log Correlation**: API / Caddy logs、PostgreSQL connections、trace_id を含む API ログ
- **Infrastructure and Object Storage**: host CPU と Garage RPC request rate

ダッシュボードを変更したら `just dashboard-sync` を再実行する。これにより UI 上の手編集を正として残さず、レビュー可能な JSON を正とする。

定義は SigNoz Dashboard v2 API を利用する。`just obs-up` は `infra/signoz/compose.dashboard-v2.yaml` を Foundry 生成 Compose に重ね、`use_dashboard_v2` を有効化してから SigNoz コンテナを再作成する。Foundry の `pours/deployment/compose.yaml` は編集しない。

## デモ用エンドポイント

`GET /api/demo/slow` — Postgres `pg_sleep(1.5)`。SigNoz で遅い SQL span を出すため。

## Postgres ログ方針

Compose の Postgres は `log_statement=mod`（変更系のみ）。`all` は使わず、認証まわりの SELECT リテラルが観測パイプラインへ過剰流入しないようにする。
