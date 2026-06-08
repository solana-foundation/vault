import react from '@vitejs/plugin-react';
import path from 'path';
import { defineConfig } from 'vite';

export default defineConfig({
    define: {
        'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV ?? 'development'),
    },
    plugins: [react()],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
        tsconfigPaths: true,
    },
    server: {
        proxy: {
            // JSON-RPC (http) -> test-validator RPC port.
            '/rpc': {
                changeOrigin: true,
                rewrite: proxyPath => proxyPath.replace(/^\/rpc/, ''),
                target: 'http://localhost:8899',
            },
            // PubSub (websocket) -> test-validator lives on rpc port + 1 (8900).
            '/rpc-ws': {
                changeOrigin: true,
                rewrite: proxyPath => proxyPath.replace(/^\/rpc-ws/, ''),
                target: 'http://localhost:8900',
                ws: true,
            },
        },
    },
});
