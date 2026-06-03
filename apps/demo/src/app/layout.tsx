import type { Metadata } from 'next';
import { Inter, JetBrains_Mono } from 'next/font/google';

import { Header } from '@/components/header';
import { Providers } from '@/components/providers';

import './globals.css';

const sans = Inter({ subsets: ['latin'], variable: '--font-geist-sans' });
const mono = JetBrains_Mono({ subsets: ['latin'], variable: '--font-geist-mono' });

export const metadata: Metadata = {
    title: 'Vault Standard Suite — Demo',
    description:
        'Interactive demo for the Solana Foundation Vault Standard Suite (async_vault). Create vaults, configure extensions, and walk through deposit/redeem flows.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
    return (
        <html lang="en" className={`${sans.variable} ${mono.variable} dark`}>
            <body>
                <Providers>
                    <Header />
                    <main className="container py-8">{children}</main>
                    <footer className="container py-12 text-center text-xs text-muted-foreground">
                        <p>
                            Built on the{' '}
                            <a
                                href="https://github.com/solana-foundation/vault"
                                className="underline-offset-4 hover:underline"
                                target="_blank"
                                rel="noreferrer"
                            >
                                Vault Standard Suite
                            </a>{' '}
                            by the Solana Foundation. This demo is unaffiliated and unaudited.
                        </p>
                    </footer>
                </Providers>
            </body>
        </html>
    );
}
