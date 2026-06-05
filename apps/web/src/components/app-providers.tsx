import { ErrorBoundary } from 'react-error-boundary';

import { RecentTransactionsProvider } from '@/contexts/RecentTransactionsContext';
import { WalletProvider } from '@/contexts/WalletContext';

import { ReactQueryProvider } from './react-query-provider';
import { SolanaProvider } from './solana/solana-provider';

function WalletErrorFallback({ error }: { error: unknown }) {
    const errorMessage = error instanceof Error ? error.message : 'An unexpected error occurred';

    function disconnectAndReload() {
        try {
            localStorage.removeItem('connector-kit:v1:account');
            localStorage.removeItem('connector-kit:v1:wallet');
            localStorage.removeItem('connector-kit:v1:wallet-state');
        } finally {
            window.location.reload();
        }
    }

    return (
        <div className="flex min-h-dvh flex-col items-center justify-center gap-4 p-8">
            <h1 className="text-2xl font-bold text-destructive">Wallet Error</h1>
            <p className="max-w-md text-center text-muted-foreground">{errorMessage}</p>
            <div className="flex gap-2">
                <button
                    className="rounded-full bg-primary px-4 py-2 text-primary-foreground hover:bg-primary/90"
                    onClick={disconnectAndReload}
                >
                    Disconnect Wallet & Reload
                </button>
                <button
                    className="rounded-full bg-secondary px-4 py-2 text-secondary-foreground hover:bg-secondary/80"
                    onClick={() => window.location.reload()}
                >
                    Reload Page
                </button>
            </div>
        </div>
    );
}

export function AppProviders({ children }: Readonly<{ children: React.ReactNode }>) {
    return (
        <ReactQueryProvider>
            <ErrorBoundary FallbackComponent={WalletErrorFallback}>
                <SolanaProvider>
                    <WalletProvider>
                        <RecentTransactionsProvider>{children}</RecentTransactionsProvider>
                    </WalletProvider>
                </SolanaProvider>
            </ErrorBoundary>
        </ReactQueryProvider>
    );
}
