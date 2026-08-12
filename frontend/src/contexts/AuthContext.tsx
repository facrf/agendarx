import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { PropsWithChildren } from "react";
import { api } from "../services/api";
import type { CredenciaisPayload, LoginResponse, UsuarioSessao } from "../types/api";

interface AuthContextValue {
  usuario: UsuarioSessao | null;
  carregando: boolean;
  login: (login: string, senha: string) => Promise<void>;
  logout: () => Promise<void>;
  atualizarCredenciais: (payload: CredenciaisPayload) => Promise<void>;
  verificarSessao: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: PropsWithChildren) {
  const [usuario, setUsuario] = useState<UsuarioSessao | null>(null);
  const [carregando, setCarregando] = useState(true);

  const verificarSessao = useCallback(async () => {
    try {
      const sessao = await api.get<{ usuario: UsuarioSessao }>("/api/auth/sessao");
      setUsuario(sessao.usuario);
    } catch {
      setUsuario(null);
    } finally {
      setCarregando(false);
    }
  }, []);

  useEffect(() => {
    void verificarSessao();
    const invalidar = () => setUsuario(null);
    window.addEventListener("agendarx:unauthorized", invalidar);
    return () => window.removeEventListener("agendarx:unauthorized", invalidar);
  }, [verificarSessao]);

  const login = useCallback(async (loginValue: string, senha: string) => {
    const resposta = await api.post<LoginResponse>("/api/auth/login", {
      login: loginValue,
      senha,
    });
    // O token do JSON não é persistido: o cookie HttpOnly emitido pela API é usado.
    setUsuario(resposta.usuario);
  }, []);

  const logout = useCallback(async () => {
    try {
      await api.post<void>("/api/auth/logout");
    } finally {
      setUsuario(null);
    }
  }, []);

  const atualizarCredenciais = useCallback(async (payload: CredenciaisPayload) => {
    await api.put("/api/auth/credenciais", payload);
    setUsuario(null);
  }, []);

  const value = useMemo(
    () => ({ usuario, carregando, login, logout, atualizarCredenciais, verificarSessao }),
    [usuario, carregando, login, logout, atualizarCredenciais, verificarSessao],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) throw new Error("useAuth deve ser usado dentro de AuthProvider");
  return context;
}
