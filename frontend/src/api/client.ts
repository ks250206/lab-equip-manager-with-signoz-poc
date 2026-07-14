export type User = {
  id: string;
  email: string;
  role: string;
};

export type Equipment = {
  id: string;
  name: string;
  description: string;
  location: string;
  image_object_key: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
};

export type Reservation = {
  id: string;
  equipment_id: string;
  user_id: string;
  starts_at: string;
  ends_at: string;
  status: string;
  created_at: string;
};

export type SessionCache = {
  user: User | null;
};

const sessionCache: SessionCache = { user: null };

export function getSessionCache(): SessionCache {
  return sessionCache;
}

export function setSessionUser(user: User | null): void {
  sessionCache.user = user;
}

export function clearSessionCache(): void {
  sessionCache.user = null;
}

export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export type ApiClientOptions = {
  baseUrl?: string;
  fetchImpl?: FetchLike;
  onUnauthorized?: () => void;
};

/**
 * Cookie-based API client.
 * On 401, attempts a single refresh then retries the original request once.
 */
export function createApiClient(options: ApiClientOptions = {}) {
  const baseUrl = options.baseUrl ?? "";
  const fetchImpl = options.fetchImpl ?? fetch.bind(globalThis);
  let refreshing: Promise<boolean> | null = null;

  async function tryRefresh(): Promise<boolean> {
    const res = await fetchImpl(`${baseUrl}/api/auth/refresh`, {
      method: "POST",
      credentials: "include",
    });
    if (!res.ok) {
      clearSessionCache();
      options.onUnauthorized?.();
      return false;
    }
    const user = (await res.json()) as User;
    setSessionUser(user);
    return true;
  }

  async function request<T>(path: string, init: RequestInit = {}, retried = false): Promise<T> {
    const headers = new Headers(init.headers);
    if (!(init.body instanceof FormData) && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }
    const res = await fetchImpl(`${baseUrl}${path}`, {
      ...init,
      credentials: "include",
      headers,
    });

    if (res.status === 401 && !retried && !path.includes("/api/auth/refresh")) {
      refreshing ??= tryRefresh().finally(() => {
        refreshing = null;
      });
      const ok = await refreshing;
      if (ok) {
        return request<T>(path, init, true);
      }
      throw new Error("unauthorized");
    }

    if (!res.ok) {
      const text = await res.text();
      throw new Error(text || `http_${res.status}`);
    }

    if (res.status === 204) {
      return undefined as T;
    }
    return (await res.json()) as T;
  }

  return {
    register(email: string, password: string) {
      return request<User>("/api/auth/register", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      }).then((user) => {
        setSessionUser(user);
        return user;
      });
    },
    login(email: string, password: string) {
      return request<User>("/api/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      }).then((user) => {
        setSessionUser(user);
        return user;
      });
    },
    logout() {
      return request<void>("/api/auth/logout", { method: "POST" }).then(() => {
        clearSessionCache();
      });
    },
    me() {
      return request<User>("/api/auth/me").then((user) => {
        setSessionUser(user);
        return user;
      });
    },
    listEquipment() {
      return request<Equipment[]>("/api/equipment");
    },
    getEquipment(id: string) {
      return request<Equipment>(`/api/equipment/${id}`);
    },
    createEquipment(body: { name: string; description?: string; location?: string }) {
      return request<Equipment>("/api/equipment", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },
    uploadEquipmentImage(id: string, file: File) {
      const form = new FormData();
      form.append("image", file);
      return request<Equipment>(`/api/equipment/${id}/image`, {
        method: "POST",
        body: form,
      });
    },
    listReservations() {
      return request<Reservation[]>("/api/reservations");
    },
    createReservation(body: { equipment_id: string; starts_at: string; ends_at: string }) {
      return request<Reservation>("/api/reservations", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },
    cancelReservation(id: string) {
      return request<Reservation>(`/api/reservations/${id}/cancel`, {
        method: "POST",
      });
    },
    slowProbe() {
      return request<{ ok: boolean }>("/api/demo/slow");
    },
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;

export function validateReservationForm(input: {
  equipmentId: string;
  startsAt: string;
  endsAt: string;
}): string | null {
  if (!input.equipmentId) return "装置を選択してください";
  if (!input.startsAt || !input.endsAt) return "開始・終了時刻を入力してください";
  const start = Date.parse(input.startsAt);
  const end = Date.parse(input.endsAt);
  if (Number.isNaN(start) || Number.isNaN(end)) return "日時の形式が不正です";
  if (end <= start) return "終了は開始より後である必要があります";
  return null;
}

export function validatePassword(password: string): string | null {
  if (password.length < 12) return "パスワードは12文字以上必要です";
  return null;
}
