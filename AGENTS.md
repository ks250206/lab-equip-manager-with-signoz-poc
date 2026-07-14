# AGENTS.md — このリポジトリで作業するエージェント向け

SigNoz 分散トレーシング検証用の **装置予約システム POC** です。実装・変更の前に本ファイルと [`doc/`](doc/) を読んでください。

## 必読ドキュメント

| 文書 | 内容 |
|------|------|
| [doc/README.md](doc/README.md) | ドキュメント索引 |
| [doc/architecture.md](doc/architecture.md) | 構成図・データ経路・ディレクトリ |
| [doc/conventions.md](doc/conventions.md) | コーディング・TDD・ツール規約 |
| [doc/auth.md](doc/auth.md) | 認証・セッション・レート制限 |
| [doc/domain.md](doc/domain.md) | 装置予約ドメイン・API |
| [doc/observability.md](doc/observability.md) | Traces / Metrics / Logs・SigNoz |
| [doc/development.md](doc/development.md) | セットアップ・just・ポート・シード |
| [frontend/AGENTS.md](frontend/AGENTS.md) | Vite+ (`vp`) 固有の注意（編集時は VITE PLUS ブロックを維持） |

## 採用スタック（変更しない）

- **App**: React (Vite+) + Axum + SQLx + PostgreSQL + Garage (S3) + Caddy
- **Obs**: OpenTelemetry SDK / tracing-opentelemetry → `otel/opentelemetry-collector-contrib` → **SigNoz** → ClickHouse
- **Runtime**: **Podman**（Compose ファイルは使うが実行エンジンは Podman。Colima / Docker Desktop 非前提）
- **Task runner**: `just`

## 採用しないもの

Grafana / Grafana Alloy / Tempo / Loki / **Prometheus サーバ** / Jaeger / Alertmanager

Garage の Prometheus 形式 metrics は OTel Collector の **prometheus receiver でスクレイプ**する（Prometheus を立てない）。

## 作業ルール（要約）

1. **古典学派 TDD**: 失敗するテストを先に書き、実装する（backend の `#[cfg(test)]` / `tests/`、frontend の `*.test.ts`）。
2. **Frontend ツール**: `vp` のみ。oxlint / oxfmt / vitest は Vite+ 経由。詳細は `frontend/AGENTS.md`。`import` は可能な限り `vite-plus` / `vite-plus/test`。
3. **コマンドは just**: `just test` / `just migrate` / `just infra-up` など。直接の長い手順は `doc/development.md`。
4. **認証仕様は厳守**: pepper + Argon2id、Cookie TTL、Refresh 回転、dummy Argon2、429+Retry-After。詳細は `doc/auth.md`。変更時はテストを更新する。
5. **観測経路を壊さない**: React `traceparent` → Axum HTTP span → service/SQLx/Garage → gateway Collector → SigNoz。ログに `trace_id` を載せる。詳細は `doc/observability.md`。
6. **スコープ**: MVP は登録/ログイン、装置 CRUD（画像は Garage）、予約作成/一覧/取消（重複は 409）。**承認フローは入れない**。
7. **計画ファイル**（`.cursor/plans/` 等）はユーザー指示がない限り編集しない。
8. **秘密情報**: `.env` をコミットしない。テンプレートは `.env.example`。

## 変更後の最低チェック

```bash
just backend-test
just frontend-test   # または frontend で vp check && vp test --run
```

観測まわりを触ったら `doc/observability.md` のスモーク手順も更新する。

## クイックスタート（詳細は doc/development.md）

```bash
cp .env.example .env
just setup && just obs-up && just migrate && just garage-init && just seed
just backend-dev   # 別ターミナル
just frontend-dev
```
