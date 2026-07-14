import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth";
import { validatePassword } from "../api/client";

export function HomePage() {
  const { user, api, setUser, loading } = useAuth();
  const navigate = useNavigate();
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (loading) return <p className="muted">読み込み中…</p>;

  if (user) {
    return (
      <section className="panel">
        <h1>装置予約システム</h1>
        <p>
          ログイン中: <strong>{user.email}</strong>（{user.role}）
        </p>
        <nav className="row">
          <Link to="/equipment">装置一覧</Link>
          <Link to="/reservations">自分の予約</Link>
          <button
            type="button"
            onClick={() => {
              void api.logout().then(() => {
                setUser(null);
                void navigate("/");
              });
            }}
          >
            ログアウト
          </button>
        </nav>
      </section>
    );
  }

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    const pwErr = validatePassword(password);
    if (mode === "register" && pwErr) {
      setError(pwErr);
      return;
    }
    setBusy(true);
    try {
      const next =
        mode === "login" ? await api.login(email, password) : await api.register(email, password);
      setUser(next);
      void navigate("/equipment");
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel">
      <h1>装置予約システム</h1>
      <p className="muted">SigNoz 分散トレーシング検証用 POC</p>
      <div className="row">
        <button
          type="button"
          className={mode === "login" ? "active" : ""}
          onClick={() => setMode("login")}
        >
          ログイン
        </button>
        <button
          type="button"
          className={mode === "register" ? "active" : ""}
          onClick={() => setMode("register")}
        >
          登録
        </button>
      </div>
      <form onSubmit={onSubmit} className="form">
        <label>
          メール
          <input
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            type="email"
            required
            autoComplete="username"
          />
        </label>
        <label>
          パスワード（12文字以上）
          <input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            required
            minLength={12}
            autoComplete={mode === "login" ? "current-password" : "new-password"}
          />
        </label>
        {error ? <p className="error">{error}</p> : null}
        <button type="submit" disabled={busy}>
          {busy ? "送信中…" : mode === "login" ? "ログイン" : "登録"}
        </button>
      </form>
    </section>
  );
}
