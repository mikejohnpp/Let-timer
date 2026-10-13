import React, { useState } from 'react';
import { KeyRound, Clock, ArrowRight, AlertCircle } from 'lucide-react';
import { setStoredToken } from '../api/client';

interface LoginViewProps {
  onLoggedIn: () => void;
}

export const LoginView: React.FC<LoginViewProps> = ({ onLoggedIn }) => {
  const [tokenInput, setTokenInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    const token = tokenInput.trim();
    if (!token) return;

    setIsLoading(true);
    setError(null);

    try {
      const res = await fetch('/api/v1/auth/me', {
        headers: {
          'Authorization': `Bearer ${token}`,
          'X-Let-Timer-Protocol': '1',
        },
      });

      if (res.ok) {
        setStoredToken(token);
        onLoggedIn();
      } else {
        const err = await res.json().catch(() => ({}));
        setError(err.message || 'Invalid or revoked API token. Please check and try again.');
      }
    } catch (e) {
      setError('Cannot connect to the Let-timer server. Please ensure the daemon is running.');
    } finally {
      setIsLoading(false);
    }
  };

  const handleGuestContinue = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const res = await fetch('/api/v1/auth/me', {
        headers: { 'X-Let-Timer-Protocol': '1' },
      });
      if (res.ok) {
        onLoggedIn();
      } else {
        setError('This server requires an API token. Please enter a token below.');
      }
    } catch (e) {
      setError('Failed to reach server.');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-[var(--bg)]">
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-3xl w-full max-w-md p-8 shadow-2xl">
        <div className="flex items-center gap-3 mb-6">
          <div className="p-3 rounded-2xl bg-[var(--accent)]/10 text-[var(--accent)]">
            <Clock className="w-8 h-8" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-[var(--text)]">Let-timer</h1>
            <p className="text-xs text-[var(--muted)]">Focus timer & daily task manager</p>
          </div>
        </div>

        <p className="text-xs text-[var(--muted)] mb-6">
          To connect to this server, paste an API token created via the CLI or web admin.
        </p>

        {error && (
          <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-500 text-xs flex items-center gap-2 mb-6">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}

        <form onSubmit={handleLogin} className="flex flex-col gap-4">
          <div>
            <label className="block text-xs font-semibold text-[var(--muted)] uppercase tracking-wider mb-2">
              API Token
            </label>
            <div className="flex items-center gap-2 bg-[var(--surface-hover)] border border-[var(--border)] rounded-2xl px-3 py-2.5 focus-within:border-[var(--accent)] transition">
              <KeyRound className="w-4 h-4 text-[var(--muted)] shrink-0" />
              <input
                type="password"
                required
                value={tokenInput}
                onChange={(e) => setTokenInput(e.target.value)}
                placeholder="Paste token here…"
                className="w-full bg-transparent text-sm text-[var(--text)] outline-none font-mono"
              />
            </div>
          </div>

          <button
            type="submit"
            disabled={isLoading || !tokenInput.trim()}
            className="w-full py-3 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white rounded-2xl font-semibold text-sm shadow-md transition flex items-center justify-center gap-2 disabled:opacity-40"
          >
            <span>{isLoading ? 'Verifying…' : 'Sign in'}</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        </form>

        <div className="mt-6 pt-6 border-t border-[var(--border)] text-center">
          <button
            onClick={handleGuestContinue}
            className="text-xs text-[var(--muted)] hover:text-[var(--text)] transition underline"
          >
            Local environment? Check if passwordless access is permitted
          </button>
        </div>
      </div>
    </div>
  );
};
