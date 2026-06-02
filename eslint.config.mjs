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
            'eslint.config.mjs',
            '**/*.mjs',
        ],
    },
];
