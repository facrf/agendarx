import { lazy, Suspense } from "react";
import type { ReactNode } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/AppShell";
import { ProtectedRoute } from "./components/ProtectedRoute";
import { Spinner } from "./components/ui";
import { LoginPage } from "./pages/LoginPage";
import { NotFoundPage } from "./pages/NotFoundPage";

const PeoplePage = lazy(() =>
  import("./pages/PeoplePage").then((module) => ({ default: module.PeoplePage })),
);
const PersonFormPage = lazy(() =>
  import("./pages/PersonFormPage").then((module) => ({ default: module.PersonFormPage })),
);
const PersonProfilePage = lazy(() =>
  import("./pages/PersonProfilePage").then((module) => ({ default: module.PersonProfilePage })),
);
const CalendarPage = lazy(() =>
  import("./pages/CalendarPage").then((module) => ({ default: module.CalendarPage })),
);
const SettingsPage = lazy(() =>
  import("./pages/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);

const GraphPage = lazy(() =>
  import("./pages/GraphPage").then((module) => ({ default: module.GraphPage })),
);

function deferredPage(page: ReactNode, label: string) {
  return <Suspense fallback={<Spinner label={label} />}>{page}</Suspense>;
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<ProtectedRoute />}>
        <Route element={<AppShell />}>
          <Route index element={<Navigate to="/pessoas" replace />} />
          <Route path="/pessoas" element={deferredPage(<PeoplePage />, "Carregando pessoas")} />
          <Route path="/pessoas/nova" element={deferredPage(<PersonFormPage />, "Carregando formulário")} />
          <Route path="/pessoas/:id" element={deferredPage(<PersonProfilePage />, "Carregando perfil")} />
          <Route path="/pessoas/:id/editar" element={deferredPage(<PersonFormPage />, "Carregando formulário")} />
          <Route path="/calendario" element={deferredPage(<CalendarPage />, "Carregando calendário")} />
          <Route path="/configuracoes" element={deferredPage(<SettingsPage />, "Carregando configurações")} />
          <Route
            path="/grafo"
            element={deferredPage(<GraphPage />, "Carregando mapa")}
          />
          <Route path="*" element={<NotFoundPage />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/pessoas" replace />} />
    </Routes>
  );
}
