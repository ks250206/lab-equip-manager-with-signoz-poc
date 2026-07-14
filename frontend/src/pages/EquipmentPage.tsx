import { useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../auth";
import type { Equipment } from "../api/client";

export function EquipmentPage() {
  const { api, user } = useAuth();
  const [items, setItems] = useState<Equipment[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [location, setLocation] = useState("");

  async function load() {
    try {
      setItems(await api.listEquipment());
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    }
  }

  useEffect(() => {
    void load();
  }, [api]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await api.createEquipment({ name, description, location });
      setName("");
      setDescription("");
      setLocation("");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    }
  }

  return (
    <section className="panel">
      <nav className="row">
        <Link to="/">ホーム</Link>
        <Link to="/reservations">予約</Link>
      </nav>
      <h1>装置一覧</h1>
      {error ? <p className="error">{error}</p> : null}
      <ul className="list">
        {items.map((eq) => (
          <li key={eq.id}>
            <Link to={`/equipment/${eq.id}`}>
              <strong>{eq.name}</strong>
            </Link>
            <span className="muted"> — {eq.location || "場所未設定"}</span>
          </li>
        ))}
      </ul>
      {user?.role === "admin" ? (
        <form className="form" onSubmit={onCreate}>
          <h2>装置を追加</h2>
          <label>
            名称
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </label>
          <label>
            説明
            <input value={description} onChange={(e) => setDescription(e.target.value)} />
          </label>
          <label>
            場所
            <input value={location} onChange={(e) => setLocation(e.target.value)} />
          </label>
          <button type="submit">作成</button>
        </form>
      ) : null}
    </section>
  );
}
