# SigNoz 装置予約システム POC

分散トレーシング検証用の **装置予約アプリケーション** です。  
React (Vite+) + Axum + PostgreSQL + Garage で業務フローを動かし、OpenTelemetry 経由で **SigNoz** に Traces / Metrics / Logs を集め、一つの UI で相関調査できることを確認します。

観測バックエンドは **SigNoz のみ** です。次は使いません。

- Grafana / Grafana Alloy
- Tempo / Loki
- Prometheus **サーバ**（Garage metrics の scrape は OTel Collector の prometheus receiver のみ）
- Jaeger / Alertmanager

## できること（MVP）

| 機能 | 内容 |
|------|------|
| 認証 | 登録 / ログイン / ログアウト / Refresh（Cookie） |
| 装置 | 一覧・詳細・作成・更新・削除（admin）・画像アップロード → Garage |
| 予約 | 作成・自分の一覧・取消。同一装置の時間重複は **409** |
| 観測デモ | 「遅いクエリ」(`pg_sleep`) で SigNoz 上の遅い Span を確認 |

予約の **承認フローはありません**。詳細は [doc/domain.md](doc/domain.md)。

## 全体の流れ

```
Browser (React OTel + traceparent)
        │
        ▼
Caddy（本番相当 :8088） / Vite proxy（開発 :5173）
        │
        ▼
Axum API（HTTP / service / SQLx / Garage spans + OTLP logs）
        │ OTLP :14317
        ▼
OTel Collector Contrib
  (OTLP / filelog / hostmetrics / postgresql / Garage scrape)
        │ OTLP :4317
        ▼
SigNoz → ClickHouse → SigNoz UI (:8080)
```

構成の詳細は [doc/architecture.md](doc/architecture.md)、観測の詳細は [doc/observability.md](doc/observability.md)。

## ドキュメント

| 文書 | 内容 |
|------|------|
| [AGENTS.md](AGENTS.md) | エージェント向け決まりごと・必読リスト |
| [doc/README.md](doc/README.md) | ドキュメント索引 |
| [doc/architecture.md](doc/architecture.md) | 構成・データ経路・ディレクトリ |
| [doc/auth.md](doc/auth.md) | 認証・Cookie・レート制限 |
| [doc/domain.md](doc/domain.md) | ドメイン・API・シード |
| [doc/observability.md](doc/observability.md) | OTel / SigNoz / 調査フロー |
| [doc/development.md](doc/development.md) | セットアップ詳細・トラブルシュート |
| [doc/conventions.md](doc/conventions.md) | TDD・ツール規約 |
| [frontend/AGENTS.md](frontend/AGENTS.md) | Vite+ (`vp`) 固有ルール |

## 必要ツール

- Rust（`cargo`）、`sqlx` CLI（migrate 用）
- [`just`](https://github.com/casey/just)
- Podman（Compose 利用）
- [`vp`](https://viteplus.dev/)（Vite+）
- SigNoz Foundry: `foundryctl`  
  `curl -fsSL https://signoz.io/foundry.sh | bash`

Podman 上で Foundry を動かす場合は、例として次のように `DOCKER_HOST` を向けます（パスは環境依存）。

```bash
export DOCKER_HOST=unix://$HOME/.local/share/containers/podman/machine/podman.sock
```

## セットアップ

```bash
cp .env.example .env

just setup          # .env 確認・frontend install・postgres/garage/otel 起動
just obs-up         # SigNoz（Foundry）
just migrate        # DB マイグレーション
just garage-init    # layout / key / bucket。表示された Key を .env の GARAGE_* に反映
just seed           # サンプルユーザー・装置・予約
```

`infra/logs/*` は Postgres / Caddy が書き込みます。`just infra-up` が権限を緩めますが、Permission denied が出たら [doc/development.md](doc/development.md) のトラブルシュートを参照してください。

### ローカル開発（推奨）

```bash
just backend-dev    # http://localhost:3000
just frontend-dev   # http://localhost:5173  （/api は Vite proxy → :3000）
```

- アプリ UI: http://localhost:5173  
- SigNoz UI: http://localhost:8080  

### Compose でまとめて起動

```bash
just infra-up-all   # backend / frontend / caddy も含めてビルド起動
```

入口は Caddy: http://localhost:8088  

## シードアカウント

| メール | パスワード | 役割 |
|--------|------------|------|
| `admin@example.com` | `admin-password-1` | admin（装置作成・画像アップロード） |
| `user@example.com` | `user-password-12` | user（予約） |

## 認証（要約）

- パスワード: `HMAC-SHA-256(PASSWORD_PEPPER)` → **Argon2id**（12 文字以上）
- Cookie: `access_token`（15 分）/ `refresh_token`（30 日）  
  `Secure`（`COOKIE_SECURE`）; `HttpOnly`; `SameSite=Lax`; `Path=/`
- DB にはトークン本体ではなく **SHA-256 ハッシュのみ**
- Access 期限切れでも Refresh 有効ならセッションは残す。Refresh 時は両方ローテーション（競合／再利用検知時はセッション破棄）
- Frontend は 401 時に Refresh を一度だけ試し、失敗時は session cache 破棄して `/` へ
- Login / Register は IP・アカウント単位のメモリ内レート制限（超過時 **429** + `Retry-After: 60`）。Caddy 配下では信頼プロキシ経由の `X-Forwarded-For` を使用

詳細は [doc/auth.md](doc/auth.md)。

## SigNoz での確認（スモーク）

1. UI でログインし、装置を予約する（または「遅いクエリを実行」）
2. SigNoz（http://localhost:8080）→ **Traces** で  
   `equipment-reservation-web` / `equipment-reservation-api` を確認
3. 遅い Span（例: `slow_probe` / `pg_sleep` / Garage put）を開く
4. 同じ **trace_id** の **Logs** で Axum の OTLP Logs や Caddy access log を突き合わせる
5. **Metrics** で host / PostgreSQL / Garage scrape などを確認

意図する調査フロー:

```
レイテンシ異常 → Trace → 遅い Span → 同 trace_id の Logs → SQL / 外部 I/O
```

## 主な just コマンド

| コマンド | 内容 |
|----------|------|
| `just obs-up` / `obs-down` | SigNoz Foundry 起動 / 停止 |
| `just infra-up` | postgres, garage, otel-collector |
| `just infra-up-all` | アプリ一式（build 含む） |
| `just infra-down` | アプリ Compose 停止 |
| `just migrate` / `seed` / `garage-init` | DB・シード・Garage |
| `just backend-dev` / `frontend-dev` | ローカル開発 |
| `just backend-test` / `frontend-test` / `test` | テスト |
| `just setup` | 初回まとめ |

一覧は `just --list` でも確認できます。

## ポート

Compose 公開は **127.0.0.1 のみ**。

| 用途 | ポート |
|------|--------|
| SigNoz UI | 8080 |
| SigNoz OTLP（gRPC / HTTP） | 4317 / 4318 |
| Gateway OTel Collector | 127.0.0.1:14317 / 14318 |
| App Caddy | 127.0.0.1:8088 |
| Backend | 127.0.0.1:3000 |
| Frontend（dev / 静的配信） | 127.0.0.1:5173 |
| Postgres | 127.0.0.1:5432 |
| Garage S3 | 127.0.0.1:3900 |
| Garage metrics | Compose 内部のみ |

## リポジトリ構成（抜粋）

```
signozpoc/
  AGENTS.md / README.md / justfile / casting.yaml
  doc/                 # 設計・運用ドキュメント
  backend/             # Axum + SQLx + OTel
  frontend/            # Vite+ React + OTel Web
  infra/               # compose, caddy, otel, garage, postgres
```

## ライセンス・位置づけ

研究・検証用の POC です。本番運用を想定したハードニングは範囲外とします。
