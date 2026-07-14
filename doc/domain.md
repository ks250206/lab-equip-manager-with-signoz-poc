# ドメイン: 装置予約

## ロール

| role | 権限 |
|------|------|
| `user` | 装置一覧/詳細の閲覧、予約の作成・自分の一覧・取消 |
| `admin` | 上記 + 装置作成・更新・削除 + 装置画像アップロード（Garage） |

承認フローはない。予約は作成時点で `active`。

## テーブル（概要）

| テーブル | 要点 |
|----------|------|
| `users` | email unique, password_hash, role |
| `sessions` | token ハッシュ、access/refresh 期限 |
| `equipment` | name, description, location, `image_object_key` |
| `reservations` | equipment_id, user_id, starts_at, ends_at, status (`active`/`cancelled`) |

マイグレーション: `backend/migrations/20260714000001_init.sql`。

## 予約ルール

- `ends_at > starts_at` 必須（違反は 400）
- 同一装置で `status = 'active'` の区間が重なる場合 **409** `reservation_conflict`
- 重複判定（半開区間イメージ）: `starts_at < other_end AND other_start < ends_at`
- ドメインヘルパ: `backend/src/domain/reservation.rs`

## HTTP API（抜粋）

| メソッド | パス | 認証 | 備考 |
|----------|------|------|------|
| GET | `/api/equipment` | 任意※ | 一覧 |
| GET | `/api/equipment/{id}` | 任意※ | 詳細 |
| POST | `/api/equipment` | admin | 作成 |
| PATCH | `/api/equipment/{id}` | admin | 名前・説明・設置場所の更新（送信した項目だけ更新） |
| DELETE | `/api/equipment/{id}` | admin | 削除（関連する予約も削除） |
| POST | `/api/equipment/{id}/image` | admin | multipart field `image` → Garage |
| GET | `/api/reservations` | 要ログイン | 自分の予約 |
| POST | `/api/reservations` | 要ログイン | 作成 |
| POST | `/api/reservations/{id}/cancel` | 要ログイン | 取消（本人・active のみ） |
| GET | `/api/demo/slow` | 要ログイン | `pg_sleep(1.5)` デモ |
| GET | `/health` | 不要 | ヘルスチェック |

※ UI はログイン必須ルートでガード。API 側の list/get equipment は POC では公開でも可。

## Garage

- バケット既定名: `equipment-images`（`GARAGE_BUCKET`）
- オブジェクトキー例: `equipment/{uuid}/{filename}`
- ブートストラップ: `just garage-init`（layout / key / bucket allow）
- トレース: `infra/garage/garage.toml` の `[admin] trace_sink`
- 管理 API は Compose ネットワーク内だけで公開する。`GARAGE_ADMIN_TOKEN` は `.env` で注入し、`just setup` が未設定時にローカル用のランダム値を生成する。

## シード

`just seed`（`backend/src/bin/seed.rs`）:

| メール | パスワード | role |
|--------|------------|------|
| admin@example.com | admin-password-1 | admin |
| user@example.com | user-password-12 | user |

装置サンプル: `CNC Mill A1` と、user の将来枠の予約 1 件。
