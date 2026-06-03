'use client';

import * as React from 'react';
import dynamic from 'next/dynamic';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Github } from 'lucide-react';

import { cn } from '@/lib/cn';
import { CLUSTER } from '@/lib/env';

const WalletMultiButton = dynamic(
    () => import('@solana/wallet-adapter-react-ui').then((m) => m.WalletMultiButton),
    { ssr: false },
);

const NAV = [
    { href: '/', label: 'Home' },
    { href: '/create', label: 'Create vault' },
    { href: '/vaults', label: 'My vaults' },
];

export function Header() {
    const pathname = usePathname();
    return (
        <header className="sticky top-0 z-40 border-b border-border/60 bg-background/70 backdrop-blur-md">
            <div className="container flex h-16 items-center gap-6">
                <Link href="/" className="group flex items-center gap-2">
                    <div className="size-8 rounded-md bg-gradient-to-br from-solana-purple to-solana-green shadow-md transition group-hover:shadow-lg" />
                    <div className="hidden flex-col leading-tight md:flex">
                        <span className="text-sm font-semibold">Vault Standard Suite</span>
                        <span className="text-[11px] text-muted-foreground">async_vault demo</span>
                    </div>
                </Link>
                <nav className="ml-4 hidden items-center gap-1 md:flex">
                    {NAV.map((n) => {
                        const active = pathname === n.href || (n.href !== '/' && pathname?.startsWith(n.href));
                        return (
                            <Link
                                key={n.href}
                                href={n.href}
                                className={cn(
                                    'rounded-md px-3 py-1.5 text-sm transition',
                                    active
                                        ? 'bg-accent text-accent-foreground'
                                        : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground',
                                )}
                            >
                                {n.label}
                            </Link>
                        );
                    })}
                </nav>
                <div className="ml-auto flex items-center gap-3">
                    <span className="hidden items-center gap-1.5 rounded-full border border-border bg-card px-2.5 py-1 text-[11px] font-mono uppercase tracking-wide text-muted-foreground md:inline-flex">
                        <span
                            className={cn(
                                'size-1.5 rounded-full',
                                CLUSTER === 'mainnet-beta'
                                    ? 'bg-red-500'
                                    : CLUSTER === 'devnet'
                                      ? 'bg-amber-500'
                                      : 'bg-blue-500',
                            )}
                        />
                        {CLUSTER}
                    </span>
                    <a
                        href="https://github.com/solana-foundation/vault"
                        target="_blank"
                        rel="noreferrer"
                        className="hidden text-muted-foreground transition hover:text-foreground sm:inline-block"
                        aria-label="GitHub"
                    >
                        <Github className="size-5" />
                    </a>
                    <WalletMultiButton style={{ height: 38, lineHeight: '38px' }} />
                </div>
            </div>
        </header>
    );
}
