/** @type {import('next').NextConfig} */
const nextConfig = {
    reactStrictMode: true,
    transpilePackages: ['@solana/vault'],
    webpack: config => {
        config.resolve.fallback = { ...config.resolve.fallback, fs: false, path: false, crypto: false };
        return config;
    },
    eslint: {
        ignoreDuringBuilds: true,
    },
};

export default nextConfig;
