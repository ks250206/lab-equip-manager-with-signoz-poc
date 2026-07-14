import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useNavigate } from "react-router-dom";
import {
  clearSessionCache,
  createApiClient,
  getSessionCache,
  type ApiClient,
  type User,
} from "./api/client";

type AuthContextValue = {
  user: User | null;
  api: ApiClient;
  loading: boolean;
  refreshMe: () => Promise<void>;
  setUser: (user: User | null) => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const navigate = useNavigate();
  const [user, setUser] = useState<User | null>(getSessionCache().user);
  const [loading, setLoading] = useState(true);

  const api = useMemo(
    () =>
      createApiClient({
        onUnauthorized: () => {
          clearSessionCache();
          setUser(null);
          void navigate("/");
        },
      }),
    [navigate],
  );

  const refreshMe = useCallback(async () => {
    try {
      const me = await api.me();
      setUser(me);
    } catch {
      setUser(null);
    }
  }, [api]);

  useEffect(() => {
    void (async () => {
      await refreshMe();
      setLoading(false);
    })();
  }, [refreshMe]);

  const value = useMemo(
    () => ({ user, api, loading, refreshMe, setUser }),
    [user, api, loading, refreshMe],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("AuthProvider missing");
  return ctx;
}
