# 開発環境

## 必要ツール

- Rust（cargo）、`sqlx` CLI（migrate 用）
- `just`
- `openssl` と `perl`（`just setup` で Garage 管理トークンを生成・保存）
- Podman（Compose プロバイダ可）
- `vp`（Vite+）
- SigNoz: `foundryctl`（`curl -fsSL https://signoz.io/foundry.sh | bash`）

Podman で Foundry を動かす場合、例:

```bash
export DOCKER_HOST=unix://$HOME/.local/share/containers/podman/machine/podman.sock
```

（環境によりソケットパスは異なる。）

## 初回セットアップ

```bash
cp .env.example .env
just setup          # frontend install + infra (postgres/garage/otel)
just obs-up         # SigNoz
just migrate
just garage-init    # 出力された Key を .env の GARAGE_* に反映
just seed
```

## 日常開発

```bash
just backend-dev    # http://localhost:3000
just frontend-dev   # http://localhost:5173  （/api → :3000 proxy）
```

Compose ですべてコンテナ化:

```bash
just infra-up-all   # 入口 Caddy http://localhost:8088
```

## Just レシピ

| コマンド | 内容 |
|----------|------|
| `just obs-up` / `obs-down` | SigNoz Foundry |
| `just infra-up` | postgres, garage, otel-collector |
| `just infra-up-all` | 上記 + backend/frontend/caddy ビルド起動 |
| `just infra-down` | アプリ Compose 停止 |
| `just migrate` | SQLx migrate |
| `just seed` | 管理者・ユーザー・装置シード |
| `just garage-init` | Garage layout / key / bucket |
| `just backend-test` / `frontend-test` / `test` | テスト |
| `just backend-dev` / `frontend-dev` | ローカル開発 |
| `just setup` | .env・install・infra-up |

## ポート一覧

Compose のホスト公開は **`127.0.0.1` のみ**（既定パスワードを LAN に晒さない）。

| 用途 | ポート |
|------|--------|
| SigNoz UI | 8080 |
| SigNoz OTLP gRPC/HTTP | 4317 / 4318 |
| Gateway OTel Collector | 127.0.0.1:14317 / 14318 |
| App Caddy | 127.0.0.1:8088 |
| Backend | 127.0.0.1:3000 |
| Frontend (dev / コンテナ静的) | 127.0.0.1:5173 |
| Postgres | 127.0.0.1:5432 |
| Garage S3 | 127.0.0.1:3900 |
| Garage web | 127.0.0.1:3902 |
| Garage metrics | Compose 内部のみ（:3903、ホスト非公開） |

## 環境変数（主要）

| 変数 | 意味 |
|------|------|
| `PASSWORD_PEPPER` | パスワード HMAC 鍵（`openssl rand -base64 32` 推奨） |
| `COOKIE_SECURE` | Cookie Secure 属性 |
| `DATABASE_URL` | アプリ用 Postgres |
| `GARAGE_*` | S3 エンドポイント・資格情報・バケット |
| `TRUSTED_PROXIES` | XFF を信頼するプロキシ CIDR（カンマ区切り） |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Backend → Gateway |
| `OTEL_SERVICE_NAME` | API の service.name |
| `FRONTEND_ORIGIN` | CORS（カンマ区切り可） |
| `SIGNOZ_OTLP_ENDPOINT` | Gateway → SigNoz（`host:4317`） |
| `VITE_OTEL_ENDPOINT` | Browser OTLP HTTP |

`GARAGE_ADMIN_TOKEN` は Garage 管理 API 用のランダムトークンで、**管理 API のポートはホスト/LAN に公開しない**。`just setup` は未設定なら `.env` に生成する。`.env` は秘密情報として共有・コミットしない。

`TRUSTED_PROXIES` は Caddy と backend 専用の `proxy` ネットワーク（既定 `172.30.0.0/24`）だけを指定する。共有 LAN や `app` ネットワーク全体を指定すると、直接接続者が `X-Forwarded-For` を偽装できる。`just setup` は旧来の広範な既定値をこの CIDR に置き換える。

## 既存DBへの予約排他制約の追加

`20260714000002_reservation_exclusion_constraint.sql` は、既存の active 予約が重複している場合に対象を変更せず明確なエラーで停止する。該当する予約を確認し、業務上不要な側を `cancelled` にしてから `just migrate` を再実行する。自動取消は行わない。

`.env.example` の Postgres / Garage 既定値はローカル POC 用です。共有 LAN にポートを開けないでください。

## トラブルシュート

### Postgres が起動しない（log Permission denied）

`infra/logs/postgres` がコンテナユーザから書けない。`just infra-up` は `chmod a+rwx` する。壊れた volume は:

```bash
podman compose -f infra/compose.yaml down -v
just infra-up
just migrate && just seed
```

### sqlx migrate: database does not exist

初期化前に volume が中途半端な状態。上記同様に volume を作り直す。

### OTel / SigNoz にデータが来ない

1. `just obs-up` で SigNoz が上がっているか（UI :8080）
2. Gateway が `14317/14318` で listen しているか
3. `SIGNOZ_OTLP_ENDPOINT` がコンテナから SigNoz に届くか（`host.containers.internal:4317`）
4. Frontend の CORS / OTLP エンドポイント（ブラウザは `14318`）

### Garage 画像アップロードが 502

`just garage-init` 未実施、または `.env` の Access/Secret が layout 後のキーと不一致。
