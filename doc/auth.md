# 認証・セッション

実装の一次情報: `backend/src/auth/`、`backend/src/api/auth.rs`、`frontend/src/api/client.ts`。

## パスワード

1. 平文パスワードを `HMAC-SHA-256(key = PASSWORD_PEPPER)` で混ぜる
2. そのバイト列を **Argon2id** でハッシュし DB の `users.password_hash` に保存
3. パスワードは **12 文字以上**（未満は 400 `password_too_short`）

既存の「連結 pepper」方式とは互換がない。方式変更後は再登録が必要。

存在しないメールでのログインでも **dummy Argon2id 検証**を実行し、ユーザー有無による応答時間差を小さくする（`dummy_verify`）。

## Cookie

ログイン / 登録成功時に設定:

| Cookie | TTL | 属性 |
|--------|-----|------|
| `access_token` | 15 分 | `Secure`（`COOKIE_SECURE`）; `HttpOnly`; `SameSite=Lax`; `Path=/` |
| `refresh_token` | 30 日 | 同上 |

ローカル開発では `COOKIE_SECURE=false` を既定とする。

## トークン保存

- トークン本体は **DB に保存しない**
- `sessions` テーブルには **SHA-256 ハッシュ**（`access_token_hash` / `refresh_token_hash`）のみ
- Access が期限切れでも、Refresh が有効なセッションは **削除しない**
- Refresh 成功時は Access と Refresh を **両方ローテーション**
- Refresh 自体が期限切れのときだけセッションを削除し、Cookie をクリア

## API

| メソッド | パス | 説明 |
|----------|------|------|
| POST | `/api/auth/register` | 登録（初期 role=`user`） |
| POST | `/api/auth/login` | ログイン |
| POST | `/api/auth/refresh` | トークン回転 |
| POST | `/api/auth/logout` | セッション削除 + Cookie クリア |
| GET | `/api/auth/me` | 要 Access Cookie |

## レート制限

Login / Register は **メモリ内**制限:

- キー: 送信元 IP（`X-Forwarded-For` / `X-Real-IP`）およびアカウント（email）
- 超過時: **429 Too Many Requests** + ヘッダ **`Retry-After: 60`**

プロセス再起動でカウンタは消える（POC 想定）。

## Frontend 挙動

1. すべての API は `credentials: 'include'`
2. **401** を受けたら `/api/auth/refresh` を **一度だけ**実行し、成功時に元リクエストを再試行
3. Refresh も失敗したら **session cache を破棄**し `/` へ遷移（`onUnauthorized`）

テスト: `frontend/src/api/client.test.ts`。
