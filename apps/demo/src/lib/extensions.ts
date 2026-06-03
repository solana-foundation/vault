import { getVaultDecoder, type Vault } from '@solana/vault';

const TLV_HEADER_SIZE = 4;

export const ExtensionType = {
    DepositFee: 1,
    WithdrawalFee: 2,
    PausableSubscriptions: 3,
    PausableRedemptions: 4,
    SubscriptionQueue: 5,
    RedemptionQueue: 6,
    MinSubscription: 7,
    MinRedemption: 8,
} as const;
export type ExtensionTypeValue = (typeof ExtensionType)[keyof typeof ExtensionType];

export const EXTENSION_LABELS: Record<ExtensionTypeValue, string> = {
    [ExtensionType.DepositFee]: 'Deposit Fee',
    [ExtensionType.WithdrawalFee]: 'Withdrawal Fee',
    [ExtensionType.PausableSubscriptions]: 'Pausable Subscriptions',
    [ExtensionType.PausableRedemptions]: 'Pausable Redemptions',
    [ExtensionType.SubscriptionQueue]: 'Subscription Queue (FIFO)',
    [ExtensionType.RedemptionQueue]: 'Redemption Queue (FIFO)',
    [ExtensionType.MinSubscription]: 'Min Subscription',
    [ExtensionType.MinRedemption]: 'Min Redemption',
};

export const EXTENSION_DESCRIPTIONS: Record<ExtensionTypeValue, string> = {
    [ExtensionType.DepositFee]: 'Charges a fee on every approved deposit, paid to the fee recipient.',
    [ExtensionType.WithdrawalFee]: 'Charges a fee on every approved redemption, paid to the fee recipient.',
    [ExtensionType.PausableSubscriptions]: 'Authority can pause/unpause new deposit requests at any time.',
    [ExtensionType.PausableRedemptions]: 'Authority can pause/unpause new redemption requests at any time.',
    [ExtensionType.SubscriptionQueue]:
        'Deposit requests are processed in FIFO order; supports cancellation tombstones.',
    [ExtensionType.RedemptionQueue]:
        'Redemption requests are processed in FIFO order; supports cancellation tombstones.',
    [ExtensionType.MinSubscription]: 'Reject deposits below a configured minimum asset amount.',
    [ExtensionType.MinRedemption]: 'Reject redemptions below a configured minimum share amount.',
};

export const EXTENSION_DATA_LEN: Record<ExtensionTypeValue, number> = {
    [ExtensionType.DepositFee]: 9,
    [ExtensionType.WithdrawalFee]: 9,
    [ExtensionType.PausableSubscriptions]: 1,
    [ExtensionType.PausableRedemptions]: 1,
    [ExtensionType.SubscriptionQueue]: 16,
    [ExtensionType.RedemptionQueue]: 16,
    [ExtensionType.MinSubscription]: 8,
    [ExtensionType.MinRedemption]: 8,
};

/**
 * Compute the byte length of the base Vault account (post-discriminator struct).
 * Done by encoding a zeroed Vault and reading the resulting byte length.
 */
export const VAULT_BASE_LEN = 272;

export type ParsedExtension =
    | { type: typeof ExtensionType.DepositFee; bps: number; lastFee: bigint }
    | { type: typeof ExtensionType.WithdrawalFee; bps: number; lastFee: bigint }
    | { type: typeof ExtensionType.PausableSubscriptions; paused: boolean }
    | { type: typeof ExtensionType.PausableRedemptions; paused: boolean }
    | { type: typeof ExtensionType.SubscriptionQueue; head: bigint; tail: bigint }
    | { type: typeof ExtensionType.RedemptionQueue; head: bigint; tail: bigint }
    | { type: typeof ExtensionType.MinSubscription; threshold: bigint }
    | { type: typeof ExtensionType.MinRedemption; threshold: bigint };

export function parseExtensions(data: Uint8Array): ParsedExtension[] {
    if (data.length <= VAULT_BASE_LEN) return [];
    const tlv = data.subarray(VAULT_BASE_LEN);
    const out: ParsedExtension[] = [];
    let offset = 0;
    const view = new DataView(tlv.buffer, tlv.byteOffset, tlv.byteLength);
    while (offset + TLV_HEADER_SIZE <= tlv.length) {
        const rawType = view.getUint16(offset, true);
        const len = view.getUint16(offset + 2, true);
        const valueStart = offset + TLV_HEADER_SIZE;
        const valueEnd = valueStart + len;
        if (rawType === 0 || valueEnd > tlv.length) break;
        const type = rawType as ExtensionTypeValue;
        const v = tlv.subarray(valueStart, valueEnd);
        const vView = new DataView(v.buffer, v.byteOffset, v.byteLength);
        switch (type) {
            case ExtensionType.DepositFee:
            case ExtensionType.WithdrawalFee: {
                const bps = vView.getUint8(0);
                const lastFee = vView.getBigUint64(1, true);
                out.push({ type, bps, lastFee });
                break;
            }
            case ExtensionType.PausableSubscriptions:
            case ExtensionType.PausableRedemptions: {
                out.push({ type, paused: vView.getUint8(0) === 1 });
                break;
            }
            case ExtensionType.SubscriptionQueue:
            case ExtensionType.RedemptionQueue: {
                const head = vView.getBigUint64(0, true);
                const tail = vView.getBigUint64(8, true);
                out.push({ type, head, tail });
                break;
            }
            case ExtensionType.MinSubscription:
            case ExtensionType.MinRedemption: {
                out.push({ type, threshold: vView.getBigUint64(0, true) });
                break;
            }
        }
        offset = valueEnd;
    }
    return out;
}

export function decodeVaultData(data: Uint8Array): Vault {
    return getVaultDecoder().decode(data.subarray(0, VAULT_BASE_LEN));
}
