import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';

import { formatTransactionError } from '@/lib/transactionErrors';

const STORAGE_KEY = 'vault-ui-recent-transactions-v1';
const MAX_RECENT_TRANSACTIONS = 20;

export type RecentTransactionValues = Record<string, string>;

export interface RecentTransaction {
    id: string;
    signature: string | null;
    action: string;
    timestamp: number;
    status: 'success' | 'failed';
    error?: string;
    values?: RecentTransactionValues;
}

interface RecentTransactionsContextType {
    recentTransactions: RecentTransaction[];
    addRecentTransaction: (transaction: RecentTransaction) => void;
    clearRecentTransactions: () => void;
}

const RecentTransactionsContext = createContext<RecentTransactionsContextType | null>(null);

function normalizeValues(values?: RecentTransactionValues): RecentTransactionValues | undefined {
    if (!values) return undefined;
    const normalizedEntries = Object.entries(values)
        .map(([key, value]) => [key, value?.trim() ?? ''] as const)
        .filter(([, value]) => value.length > 0);
    if (normalizedEntries.length === 0) return undefined;
    return Object.fromEntries(normalizedEntries);
}

function readStoredTransactions(): RecentTransaction[] {
    try {
        const raw = window.localStorage.getItem(STORAGE_KEY);
        if (!raw) return [];
        const parsed: unknown = JSON.parse(raw);
        if (!Array.isArray(parsed)) return [];
        return (parsed as RecentTransaction[]).slice(0, MAX_RECENT_TRANSACTIONS);
    } catch {
        return [];
    }
}

export function RecentTransactionsProvider({ children }: { children: React.ReactNode }) {
    const [recentTransactions, setRecentTransactions] = useState<RecentTransaction[]>([]);
    const [hydrated, setHydrated] = useState(false);

    useEffect(() => {
        setRecentTransactions(readStoredTransactions());
        setHydrated(true);
    }, []);

    useEffect(() => {
        if (!hydrated) return;
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(recentTransactions));
    }, [hydrated, recentTransactions]);

    const addRecentTransaction = useCallback((transaction: RecentTransaction) => {
        setRecentTransactions(current => {
            const normalized: RecentTransaction = {
                ...transaction,
                id: transaction.id.trim() || `${Date.now()}`,
                signature: transaction.signature?.trim() || null,
                action: transaction.action.trim() || 'Transaction',
                error: transaction.error?.trim() ? formatTransactionError(transaction.error) : undefined,
                values: normalizeValues(transaction.values),
            };
            const deduped = current.filter(item =>
                normalized.signature ? item.signature !== normalized.signature : item.id !== normalized.id,
            );
            return [normalized, ...deduped].slice(0, MAX_RECENT_TRANSACTIONS);
        });
    }, []);

    const clearRecentTransactions = useCallback(() => setRecentTransactions([]), []);

    const value = useMemo(
        () => ({ recentTransactions, addRecentTransaction, clearRecentTransactions }),
        [recentTransactions, addRecentTransaction, clearRecentTransactions],
    );

    return <RecentTransactionsContext.Provider value={value}>{children}</RecentTransactionsContext.Provider>;
}

export function useRecentTransactions() {
    const context = useContext(RecentTransactionsContext);
    if (!context) {
        throw new Error('useRecentTransactions must be used inside RecentTransactionsProvider');
    }
    return context;
}
