import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState
} from 'react';

export type Role = 'employee' | 'manager' | 'finance' | 'admin';

interface AuthState {
  token?: string;
  role: Role;
}

interface AuthContextValue extends AuthState {
  isAuthenticated: boolean;
  isReady: boolean;
  login: (token: string, role: Role) => void;
  logout: () => void;
}

export const AUTH_TOKEN_STORAGE_KEY = 'fsi-auth-token';
export const AUTH_LOGOUT_EVENT = 'fsi-auth-logout';

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [token, setToken] = useState<string | undefined>();
  const [role, setRole] = useState<Role>('employee');
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    const storedToken = localStorage.getItem(AUTH_TOKEN_STORAGE_KEY);
    if (storedToken) {
      setToken(storedToken);
    }
    setIsReady(true);

    const handleStorage = (event: StorageEvent) => {
      if (event.key === AUTH_TOKEN_STORAGE_KEY) {
        if (event.newValue) {
          setToken(event.newValue);
        } else {
          setToken(undefined);
          setRole('employee');
        }
      }
    };

    const handleLogoutEvent = () => {
      setToken(undefined);
      setRole('employee');
    };

    window.addEventListener('storage', handleStorage);
    window.addEventListener(AUTH_LOGOUT_EVENT, handleLogoutEvent);

    return () => {
      window.removeEventListener('storage', handleStorage);
      window.removeEventListener(AUTH_LOGOUT_EVENT, handleLogoutEvent);
    };
  }, []);

  const login = useCallback((nextToken: string, nextRole: Role) => {
    localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, nextToken);
    setToken(nextToken);
    setRole(nextRole);
  }, []);

  const logout = useCallback(() => {
    localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
    setToken(undefined);
    setRole('employee');
    window.dispatchEvent(new Event(AUTH_LOGOUT_EVENT));
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      token,
      role,
      isAuthenticated: Boolean(token),
      isReady,
      login,
      logout
    }),
    [token, role, isReady, login, logout]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};
