const KEY = 'vault-suite-demo:known-vaults:v1';

export interface KnownVault {
    shareMint: string;
    assetMint: string;
    label?: string;
    createdAt: number;
    /** Whether the connected wallet is the authority. Best-effort hint, not authoritative. */
    isAuthority?: boolean;
    assetTokenProgram?: string;
    shareTokenProgram?: string;
    /** Pretend asset mint authority kept by the demo when the user spins up a synthetic asset. */
    demoAssetMintAuthority?: string;
}

export function getKnownVaults(): KnownVault[] {
    if (typeof window === 'undefined') return [];
    try {
        const raw = window.localStorage.getItem(KEY);
        if (!raw) return [];
        const parsed = JSON.parse(raw) as KnownVault[];
        return Array.isArray(parsed) ? parsed : [];
    } catch {
        return [];
    }
}

export function saveKnownVault(v: KnownVault): void {
    if (typeof window === 'undefined') return;
    const existing = getKnownVaults().filter(x => x.shareMint !== v.shareMint);
    const next = [v, ...existing].slice(0, 50);
    window.localStorage.setItem(KEY, JSON.stringify(next));
}

export function removeKnownVault(shareMint: string): void {
    if (typeof window === 'undefined') return;
    const next = getKnownVaults().filter(x => x.shareMint !== shareMint);
    window.localStorage.setItem(KEY, JSON.stringify(next));
}

export function updateKnownVault(shareMint: string, patch: Partial<KnownVault>): void {
    const list = getKnownVaults();
    const next = list.map(v => (v.shareMint === shareMint ? { ...v, ...patch } : v));
    if (typeof window !== 'undefined') {
        window.localStorage.setItem(KEY, JSON.stringify(next));
    }
}
