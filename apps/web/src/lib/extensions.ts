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
                out.push({ bps, lastFee, type });
                break;
            }
            case ExtensionType.PausableSubscriptions:
            case ExtensionType.PausableRedemptions: {
                out.push({ paused: vView.getUint8(0) === 1, type });
                break;
            }
            case ExtensionType.SubscriptionQueue:
            case ExtensionType.RedemptionQueue: {
                const head = vView.getBigUint64(0, true);
                const tail = vView.getBigUint64(8, true);
                out.push({ head, tail, type });
                break;
            }
            case ExtensionType.MinSubscription:
            case ExtensionType.MinRedemption: {
                out.push({ threshold: vView.getBigUint64(0, true), type });
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
