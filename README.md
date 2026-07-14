# SigNoz 装置予約システム POC

React (Vite+) + Axum + PostgreSQL + Garage + Caddy + OpenTelemetry Collector Contrib。観測バックエンドは **SigNoz** のみ（Grafana / Tempo / Loki / Prometheus サーバ等は不使用）。

## ドキュメント

| 文書 | 内容 |
|------|------|
| [AGENTS.md](AGENTS.md) | エージェント向け決まりごと・必読リスト |
| [doc/](doc/README.md) | アーキテクチャ・認証・ドメイン・観測・開発手順 |
| [frontend/AGENTS.md](frontend/AGENTS.md) | Vite+ (`vp`) 固有ルール |

## 最短セットアップ

```bash
cp .env.example .env
just setup && just obs-up && just migrate && just garage-init && just seed
just backend-dev    # :3000
just frontend-dev   # :5173
```

詳細・ポート・トラブルシュートは [doc/development.md](doc/development.md)。  
シードアカウント・スモーク手順は [doc/domain.md](doc/domain.md) / [doc/observability.md](doc/observability.md)。
