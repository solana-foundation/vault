'use client';

import * as React from 'react';
import dynamic from 'next/dynamic';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { Toaster } from 'sonner';

import { CLUSTER, RPC_URL } from '@/lib/env';

const WalletModalProvider = dynamic(() => import('@solana/wallet-adapter-react-ui').then(m => m.WalletModalProvider), {
    ssr: false,
});

import '@solana/wallet-adapter-react-ui/styles.css';

export function Providers({ children }: { children: React.ReactNode }) {
    return (
        <ConnectionProvider endpoint={RPC_URL}>
            <WalletProvider wallets={[]} autoConnect>
                <WalletModalProvider>
                    {children}
                    <Toaster
                        position="bottom-right"
                        theme="dark"
                        toastOptions={{
                            classNames: {
                                toast: 'group toast group-[.toaster]:bg-card group-[.toaster]:text-card-foreground group-[.toaster]:border-border group-[.toaster]:shadow-lg',
                                description: 'group-[.toast]:text-muted-foreground',
                            },
                        }}
                    />
                    <ClusterBadge cluster={CLUSTER} />
                </WalletModalProvider>
            </WalletProvider>
        </ConnectionProvider>
    );
}

function ClusterBadge({ cluster }: { cluster: string }) {
    const color = cluster === 'mainnet-beta' ? 'bg-red-500' : cluster === 'devnet' ? 'bg-amber-500' : 'bg-blue-500';
    return (
        <div className="fixed bottom-4 left-4 z-50 hidden items-center gap-2 rounded-full bg-card/80 px-3 py-1 text-xs font-mono shadow-lg backdrop-blur md:flex">
            <span className={`size-2 rounded-full ${color}`} />
            <span className="text-muted-foreground">cluster</span>
            <span className="text-foreground">{cluster}</span>
        </div>
    );
}
