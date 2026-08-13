import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { ProtectedRoute } from "./components/ProtectedRoute";
import { Spinner } from "./components/ui";
import { LoginPage } from "./pages/LoginPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { PeoplePage } from "./pages/PeoplePage";
import { PersonFormPage } from "./pages/PersonFormPage";
import { PersonProfilePage } from "./pages/PersonProfilePage";
import { SettingsPage } from "./pages/SettingsPage";
import { CalendarPage } from "./pages/CalendarPage";

const GraphPage = lazy(() =>
  import("./pages/GraphPage").then((module) => ({ default: module.GraphPage })),
);

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<ProtectedRoute />}>
        <Route element={<AppShell />}>
          <Route index element={<Navigate to="/pessoas" replace />} />
          <Route path="/pessoas" element={<PeoplePage />} />
          <Route path="/pessoas/nova" element={<PersonFormPage />} />
          <Route path="/pessoas/:id" element={<PersonProfilePage />} />
          <Route path="/pessoas/:id/editar" element={<PersonFormPage />} />
          <Route path="/calendario" element={<CalendarPage />} />
          <Route path="/configuracoes" element={<SettingsPage />} />
          <Route
            path="/grafo"
            element={
              <Suspense fallback={<Spinner label="Carregando mapa" />}>
                <GraphPage />
              </Suspense>
            }
          />
          <Route path="*" element={<NotFoundPage />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/pessoas" replace />} />
    </Routes>
  );
}
