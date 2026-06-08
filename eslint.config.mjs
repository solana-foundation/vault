import solana from '@solana/eslint-config-solana';

export default [
    ...solana,
    {
        ignores: [
            '**/dist/**',
            '**/node_modules/**',
            '**/target/**',
            '**/generated/**',
            'clients/rust/**',
            'clients/typescript/src/generated/**',
            '.remember/**',
            '.claude/**',
            'eslint.config.mjs',
            '**/*.mjs',
            // The Next.js demo has its own ESLint config (next/core-web-vitals).
            // It uses async event handlers, browser-side decoded account data, etc.
            // that conflict with this strict, type-aware Solana program lint preset.
            'apps/**',
        ],
    },
];
