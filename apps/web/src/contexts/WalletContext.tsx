import { createContext, useCallback, useContext, useMemo } from 'react';
import type { TransactionSigner } from '@solana/kit';
import { useKitTransactionSigner, useWallet as useConnectorWallet } from '@solana/connector/react';

interface WalletAccount {
    address: string;
}

interface WalletContextType {
    account: WalletAccount | null;
    connected: boolean;
    connecting: boolean;
    createSigner: () => TransactionSigner | null;
}

const WalletContext = createContext<WalletContextType | null>(null);

export function WalletProvider({ children }: { children: React.ReactNode }) {
    const { account: connectorAccount, isConnected, isConnecting } = useConnectorWallet();
    const { signer } = useKitTransactionSigner();

    const account = useMemo<WalletAccount | null>(
        () => (connectorAccount ? { address: connectorAccount } : null),
        [connectorAccount],
    );

    const createSigner = useCallback((): TransactionSigner | null => signer ?? null, [signer]);

    const value = useMemo(
        () => ({ account, connected: isConnected, connecting: isConnecting, createSigner }),
        [account, createSigner, isConnected, isConnecting],
    );

    return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet() {
    const ctx = useContext(WalletContext);
    if (!ctx) throw new Error('useWallet must be used inside WalletProvider');
    return ctx;
}
