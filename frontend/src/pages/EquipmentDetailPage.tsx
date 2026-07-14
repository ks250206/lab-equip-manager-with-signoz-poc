import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { useAuth } from "../auth";
import { validateReservationForm, type Equipment } from "../api/client";

export function EquipmentDetailPage() {
  const { id = "" } = useParams();
  const { api, user } = useAuth();
  const [equipment, setEquipment] = useState<Equipment | null>(null);
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [file, setFile] = useState<File | null>(null);

  useEffect(() => {
    void api
      .getEquipment(id)
      .then(setEquipment)
      .catch((err: unknown) => setError(err instanceof Error ? err.message : "failed"));
  }, [api, id]);

  async function onReserve(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setMessage(null);
    const validation = validateReservationForm({
      equipmentId: id,
      startsAt,
      endsAt,
    });
    if (validation) {
      setError(validation);
      return;
    }
    try {
      await api.createReservation({
        equipment_id: id,
        starts_at: new Date(startsAt).toISOString(),
        ends_at: new Date(endsAt).toISOString(),
      });
      setMessage("予約を作成しました");
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    }
  }

  async function onUpload(e: FormEvent) {
    e.preventDefault();
    if (!file) return;
    setError(null);
    try {
      const updated = await api.uploadEquipmentImage(id, file);
      setEquipment(updated);
      setMessage("画像をアップロードしました（Garage）");
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed");
    }
  }

  if (!equipment && !error) return <p className="muted">読み込み中…</p>;
  if (!equipment) return <p className="error">{error}</p>;

  return (
    <section className="panel">
      <nav className="row">
        <Link to="/equipment">一覧へ</Link>
        <Link to="/reservations">予約</Link>
      </nav>
      <h1>{equipment.name}</h1>
      <p>{equipment.description || "説明なし"}</p>
      <p className="muted">場所: {equipment.location || "未設定"}</p>
      {equipment.image_object_key ? (
        <p className="muted">画像キー: {equipment.image_object_key}</p>
      ) : null}
      {error ? <p className="error">{error}</p> : null}
      {message ? <p className="ok">{message}</p> : null}

      {user ? (
        <form className="form" onSubmit={onReserve}>
          <h2>予約する</h2>
          <label>
            開始
            <input
              type="datetime-local"
              value={startsAt}
              onChange={(e) => setStartsAt(e.target.value)}
              required
            />
          </label>
          <label>
            終了
            <input
              type="datetime-local"
              value={endsAt}
              onChange={(e) => setEndsAt(e.target.value)}
              required
            />
          </label>
          <button type="submit">予約作成</button>
        </form>
      ) : null}

      {user?.role === "admin" ? (
        <form className="form" onSubmit={onUpload}>
          <h2>画像アップロード（Garage）</h2>
          <input
            type="file"
            accept="image/*"
            onChange={(e) => setFile(e.target.files?.[0] ?? null)}
          />
          <button type="submit" disabled={!file}>
            アップロード
          </button>
        </form>
      ) : null}
    </section>
  );
}
