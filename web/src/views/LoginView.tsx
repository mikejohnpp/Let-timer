import React, { useState } from 'react';
import { Clock, ArrowRight, Eye, EyeOff } from 'lucide-react';
import { setStoredToken } from '../api/client';
import { Card } from '../components/ui/Card';
import { FormField, Input } from '../components/ui/Field';
import { Button } from '../components/ui/Button';
import { Banner } from '../components/ui/Banner';

interface LoginViewProps {
  onLoggedIn: () => void;
}

export const LoginView: React.FC<LoginViewProps> = ({ onLoggedIn }) => {
  const [tokenInput, setTokenInput] = useState('');
  const [showToken, setShowToken] = useState(false);
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
          Authorization: `Bearer ${token}`,
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
    } catch {
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
    } catch {
      setError('Failed to reach server.');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-[var(--bg)]">
      <Card className="w-full max-w-md p-6 sm:p-8 shadow-2xl border-[var(--border)]">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2.5 rounded-[var(--radius-card)] bg-[var(--accent-tint)] text-[var(--accent-text)]">
            <Clock className="w-7 h-7" />
          </div>
          <div>
            <h1 className="text-xl font-semibold leading-tight text-[var(--text)]">Let-timer</h1>
            <p className="text-xs text-[var(--muted)]">Focus timer & daily task manager</p>
          </div>
        </div>

        <p className="text-xs text-[var(--muted)] mb-6">
          To connect to this server, paste an API token created via the CLI or web admin.
        </p>

        {error && (
          <Banner tone="danger" className="mb-4">
            {error}
          </Banner>
        )}

        <form onSubmit={handleLogin} className="flex flex-col gap-4">
          <FormField label="API Token" required>
            {(id, describedBy) => (
              <div className="relative flex items-center">
                <Input
                  id={id}
                  aria-describedby={describedBy}
                  type={showToken ? 'text' : 'password'}
                  required
                  value={tokenInput}
                  onChange={(e) => setTokenInput(e.target.value)}
                  placeholder="Paste token here…"
                  className="w-full pr-10 font-mono"
                  autoFocus
                />
                <button
                  type="button"
                  onClick={() => setShowToken(!showToken)}
                  className="absolute right-2 text-[var(--muted)] hover:text-[var(--text)] p-1 rounded focus-visible:outline-2 focus-visible:outline-[var(--accent-text)]"
                  aria-label={showToken ? 'Hide token' : 'Show token'}
                >
                  {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            )}
          </FormField>

          <Button
            type="submit"
            variant="primary"
            disabled={isLoading || !tokenInput.trim()}
            className="w-full justify-center mt-2"
          >
            <span>{isLoading ? 'Verifying…' : 'Sign in'}</span>
            <ArrowRight className="w-4 h-4" />
          </Button>
        </form>

        <div className="mt-6 pt-6 border-t border-[var(--border)] text-center">
          <button
            type="button"
            onClick={handleGuestContinue}
            disabled={isLoading}
            className="text-xs text-[var(--muted)] hover:text-[var(--text)] transition underline cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--accent-text)]"
          >
            Local environment? Check if passwordless access is permitted
          </button>
        </div>
      </Card>
    </div>
  );
};
