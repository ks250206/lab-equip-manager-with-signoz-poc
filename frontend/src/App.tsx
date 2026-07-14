import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import type { ReactNode } from "react";
import { AuthProvider, useAuth } from "./auth";
import { HomePage } from "./pages/HomePage";
import { EquipmentPage } from "./pages/EquipmentPage";
import { EquipmentDetailPage } from "./pages/EquipmentDetailPage";
import { ReservationsPage } from "./pages/ReservationsPage";

function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return <p className="muted">読み込み中…</p>;
  if (!user) return <Navigate to="/" replace />;
  return children;
}

export function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <main className="app">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route
              path="/equipment"
              element={
                <RequireAuth>
                  <EquipmentPage />
                </RequireAuth>
              }
            />
            <Route
              path="/equipment/:id"
              element={
                <RequireAuth>
                  <EquipmentDetailPage />
                </RequireAuth>
              }
            />
            <Route
              path="/reservations"
              element={
                <RequireAuth>
                  <ReservationsPage />
                </RequireAuth>
              }
            />
          </Routes>
        </main>
      </AuthProvider>
    </BrowserRouter>
  );
}
