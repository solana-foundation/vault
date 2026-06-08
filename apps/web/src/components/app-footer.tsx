export function AppFooter() {
    return (
        <footer className="mx-auto w-full max-w-7xl px-6 py-12 text-center text-xs text-muted-foreground">
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
    );
}
