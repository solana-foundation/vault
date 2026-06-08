export function shortAddress(addr: string | null | undefined, chars = 4): string {
    if (!addr) return '';
    if (addr.length <= chars * 2 + 3) return addr;
    return `${addr.slice(0, chars)}…${addr.slice(-chars)}`;
}

export function formatTokenAmount(raw: bigint | number | null | undefined, decimals: number): string {
    if (raw == null) return '—';
    const n = typeof raw === 'bigint' ? raw : BigInt(raw);
    const sign = n < 0n ? '-' : '';
    const abs = n < 0n ? -n : n;
    const divisor = 10n ** BigInt(decimals);
    const whole = abs / divisor;
    const frac = abs % divisor;
    if (frac === 0n) return `${sign}${whole.toLocaleString('en-US')}`;
    const fracStr = frac.toString().padStart(decimals, '0').replace(/0+$/, '').slice(0, 6);
    return `${sign}${whole.toLocaleString('en-US')}${fracStr ? '.' + fracStr : ''}`;
}

export function parseTokenAmount(input: string, decimals: number): bigint {
    const s = input.trim();
    if (!s) return 0n;
    if (!/^\d+(\.\d+)?$/.test(s)) throw new Error(`Invalid amount: ${input}`);
    const [whole, fracRaw = ''] = s.split('.');
    const frac = fracRaw.padEnd(decimals, '0').slice(0, decimals);
    return BigInt(whole + frac);
}
