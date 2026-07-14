# 決まりごと・規約

## プロダクト方針

- 本リポジトリは **観測スタック検証 POC** 兼 **装置予約 MVP**。
- MVP に含めない: 予約承認フロー、通知、繰り返し予約、Grafana 系スタック。
- オブジェクトストレージは必要時 **Garage**（S3 互換 + OTel）を使う。

## 古典学派 TDD

1. 失敗するテストを書く（Red）
2. 最小実装で通す（Green）
3. 整理する（Refactor）

| 領域 | 置き場所の例 |
|------|----------------|
| Backend ドメイン | `backend/src/**/tests` モジュール、`backend/tests/` |
| Frontend クライアント | `frontend/src/**/*.test.ts` |

必須カバレッジの例: password pepper+Argon2、セッション TTL/回転、レート制限、予約時間重複、API client の 401→refresh→retry。

## Frontend（Vite+）

- エントリは **`vp`**。pnpm/npm/yarn を直接叩かない（詳細は `frontend/AGENTS.md`）。
- フォーマット / lint / typecheck: `vp check`
- テスト: `vp test`（CI・just では `vp test --run`）
- oxlint / oxfmt / Vitest は Vite+ 同梱を使う。独自に ESLint/Prettier を足さない。
- 設定は `frontend/vite.config.ts`（`defineConfig` from `vite-plus`）。
- React Router + Cookie 認証（`credentials: 'include'`）。

## Backend（Rust）

- クレート: `equipment_reservation`（`backend/`）
- モジュール目安: `auth` / `domain` / `infra` / `api` / `telemetry`
- 非同期ランタイム: Tokio、HTTP: Axum 0.8、DB: SQLx + migrate
- ログ: `tracing` + JSON subscriber。OTel: `tracing-opentelemetry` + OTLP
- エラー応答は JSON `{ "error": "..." }` を基本とする

## インフラ

- コンテナは **Podman**（`podman compose`）。SigNoz のみ Foundry（内部は Compose）。
- 秘密・接続情報は `.env`（`.env.example` をコピー）。コミット禁止。
- `infra/logs/*` はコンテナ書き込み用。`just infra-up` が権限を緩める。

## コミット・PR

- ユーザー依頼があるまで commit / push しない。
- `.env`、資格情報、`pours/`（Foundry 生成物）をコミットしない。

## ドキュメントの更新義務

仕様・経路・ポート・コマンドを変えたら、対応する `doc/*.md` と必要なら `AGENTS.md` / `README.md` を同じ変更で更新する。
