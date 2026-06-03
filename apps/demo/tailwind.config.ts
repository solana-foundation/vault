import type { Config } from 'tailwindcss';
import animate from 'tailwindcss-animate';

const config: Config = {
    darkMode: 'class',
    content: ['./src/**/*.{ts,tsx}'],
    theme: {
        container: {
            center: true,
            padding: '1rem',
            screens: {
                '2xl': '1400px',
            },
        },
        extend: {
            fontFamily: {
                sans: ['var(--font-geist-sans)', 'system-ui', 'sans-serif'],
                mono: ['var(--font-geist-mono)', 'ui-monospace', 'monospace'],
            },
            colors: {
                background: 'hsl(var(--background))',
                foreground: 'hsl(var(--foreground))',
                card: {
                    DEFAULT: 'hsl(var(--card))',
                    foreground: 'hsl(var(--card-foreground))',
                },
                muted: {
                    DEFAULT: 'hsl(var(--muted))',
                    foreground: 'hsl(var(--muted-foreground))',
                },
                accent: {
                    DEFAULT: 'hsl(var(--accent))',
                    foreground: 'hsl(var(--accent-foreground))',
                },
                primary: {
                    DEFAULT: 'hsl(var(--primary))',
                    foreground: 'hsl(var(--primary-foreground))',
                },
                destructive: {
                    DEFAULT: 'hsl(var(--destructive))',
                    foreground: 'hsl(var(--destructive-foreground))',
                },
                success: {
                    DEFAULT: 'hsl(var(--success))',
                    foreground: 'hsl(var(--success-foreground))',
                },
                warning: {
                    DEFAULT: 'hsl(var(--warning))',
                    foreground: 'hsl(var(--warning-foreground))',
                },
                border: 'hsl(var(--border))',
                ring: 'hsl(var(--ring))',
                input: 'hsl(var(--input))',
                solana: {
                    purple: '#9945FF',
                    green: '#14F195',
                    blue: '#19FB9B',
                },
            },
            borderRadius: {
                lg: 'var(--radius)',
                md: 'calc(var(--radius) - 2px)',
                sm: 'calc(var(--radius) - 4px)',
            },
            keyframes: {
                'fade-in': {
                    '0%': { opacity: '0', transform: 'translateY(4px)' },
                    '100%': { opacity: '1', transform: 'translateY(0)' },
                },
                shimmer: {
                    '0%': { backgroundPosition: '0% 50%' },
                    '100%': { backgroundPosition: '100% 50%' },
                },
            },
            animation: {
                'fade-in': 'fade-in 200ms ease-out',
                shimmer: 'shimmer 8s ease-in-out infinite',
            },
        },
    },
    plugins: [animate],
};

export default config;
