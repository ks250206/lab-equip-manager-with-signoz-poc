import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../auth";
import type { Reservation } from "../api/client";

export function ReservationsPage() {
  const { api } = useAuth();
  const [items, setItems] = useState<Reservation[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [slowMsg, setSlowMsg] = useState<string | null>(null);

  async function load() {
    try {
      setItems(await api.listReservations());
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    }
  }

  useEffect(() => {
    void load();
  }, [api]);

  return (
    <section className="panel">
      <nav className="row">
        <Link to="/">ホーム</Link>
        <Link to="/equipment">装置</Link>
      </nav>
      <h1>自分の予約</h1>
      {error ? <p className="error">{error}</p> : null}
      <ul className="list">
        {items.map((r) => (
          <li key={r.id}>
            <div>
              <strong>{r.status}</strong> / 装置 {r.equipment_id.slice(0, 8)}…
              <br />
              <span className="muted">
                {new Date(r.starts_at).toLocaleString()} → {new Date(r.ends_at).toLocaleString()}
              </span>
            </div>
            {r.status === "active" ? (
              <button
                type="button"
                onClick={() => {
                  void api.cancelReservation(r.id).then(load);
                }}
              >
                取消
              </button>
            ) : null}
          </li>
        ))}
      </ul>
      <button
        type="button"
        onClick={() => {
          setSlowMsg(null);
          void api
            .slowProbe()
            .then(() => setSlowMsg("slow probe 完了（SigNoz で遅い SQL span を確認）"))
            .catch((err: unknown) => setError(err instanceof Error ? err.message : "failed"));
        }}
      >
        遅いクエリを実行（デモ）
      </button>
      {slowMsg ? <p className="ok">{slowMsg}</p> : null}
    </section>
  );
}
