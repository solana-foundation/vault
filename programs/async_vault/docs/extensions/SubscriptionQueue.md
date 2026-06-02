# Queue Extensions: SubscriptionQueue & RedemptionQueue

## Overview

The `SubscriptionQueue` and `RedemptionQueue` extensions each enforce **first-in, first-out (FIFO) ordering** for their respective request type. When active on a vault, the authority must approve or reject requests in the exact order they were submitted — the oldest pending request must always be processed before any newer ones.

- **SubscriptionQueue** applies to deposit requests.
- **RedemptionQueue** applies to redeem requests.

Both extensions use identical queue mechanics. The sections below describe the shared design; differences between the two are called out explicitly.

---

## How It Works

### Counters

Each extension stores two numbers on the vault account:

| Field                   | SubscriptionQueue                           | RedemptionQueue                           |
| ----------------------- | ------------------------------------------- | ----------------------------------------- |
| All-time total requests | `all_time_total_subscription_requests`      | `all_time_total_redemption_requests`      |
| Last processed index    | `last_processed_subscription_request_index` | `last_processed_redemption_request_index` |

When a user submits a request, the vault's counter is incremented and the new value is stamped onto the request account as its **ID**. For example, the first request ever gets ID `1`, the second gets `2`, and so on.

### Enforcement

When the vault authority tries to approve or reject a request, the program checks:

```
request.id == last_processed + 1
```

If the IDs don't match — meaning an earlier request is still pending — the transaction fails with `SubscriptionQueueOutOfOrder` or `RedemptionQueueOutOfOrder` respectively. This makes it impossible to process requests out of turn.

After a request is successfully approved or rejected, `last_processed` advances to that request's ID.

---

## Lifecycle of a Request

```
User submits request
        │
        ▼
Request created → assigned ID (counter + 1)
        │
        ├──► Authority approves or rejects (in order) → last_processed advances → account closed
        │
        └──► User cancels → refunded immediately* → account stays open as tombstone
                    │
                    ▼
             Anyone calls skip_canceled → queue advances → tombstone closed, rent returned
```

\* On cancel: SubscriptionQueue transfers deposited assets back; RedemptionQueue mints burned shares back.

---

## Canceling a Queued Request

### The Problem

Canceling a request is straightforward from the user's perspective — they want their assets (or shares) back. But in a FIFO queue, a canceled request leaves a **gap** in the ID sequence. If that gap is never resolved, the queue gets permanently stuck: the next request has ID `N+2`, but the queue is still waiting for `N+1` which no longer exists.

A naive fix, closing the account immediately and marking the queue as advanced, creates a different problem: a malicious user could cancel requests at will to stall the queue, or worse, loop through many rapid create-and-cancel cycles to overflow a fixed-size tracking structure. This makes the queue permanently unrecoverable at negligible cost.

### The Solution: Tombstone Accounts

When a user cancels a queued request:

1. **The user is refunded immediately** — deposited assets are transferred back (SubscriptionQueue) or burned shares are minted back (RedemptionQueue).
2. **The request account stays open** in a `Canceled` state rather than being closed.

The open account acts as a **tombstone** — a lightweight marker that proves this ID existed and was properly canceled. No queue state is modified yet; the tombstone just sits there until the queue naturally reaches it.

Because accounts cost rent (a small ongoing fee paid in SOL), keeping them open is not free. This prevents spam: an attacker trying to create many tombstones to grief the queue must pay rent for every open account they leave behind, and the queue can always be unblocked.

### Why Not Use `cancel_request`?

The existing `cancel_request` instruction closes the account entirely when it finishes. Using it on a queued request would destroy the tombstone, recreating the stuck-queue bug. To prevent this, `cancel_request` now returns an error (`MustUseCancelQueuedDepositRequest`) if called on a request that belongs to a queue-enabled vault. Callers should use `cancel_queued_deposit_request` or `cancel_queued_redemption_request` instead.

Requests on non-queued vaults are unaffected — `cancel_request` still closes their accounts immediately as before.

---

## Advancing Past a Tombstone

### `skip_canceled_queue_request`

Once the queue has processed all earlier requests and arrives at a tombstone, someone must tell the program to skip over it. This is done by calling `skip_canceled_queue_request`.

**This instruction is permissionless** — anyone can call it, not just the vault authority or the original requester. This is intentional: if the vault authority is slow to act, the user (or anyone else) can unblock the queue themselves.

When called, the instruction:

1. Verifies the request is in `Canceled` state and is the next expected ID (`last_processed + 1`).
2. Advances `last_processed` to this request's ID.
3. Closes the tombstone account and **returns the rent to the original requester**.

Multiple consecutive tombstones must each be skipped with a separate call, in order. Each call advances the queue by one position.

### Example: Cancel in the Middle of a Queue

Suppose three requests exist with IDs 1, 2, and 3, and request 2 is canceled:

```
Step 1: Cancel request #2
        → user refunded, account stays open (tombstone)
        → last_processed = 0

Step 2: Authority approves request #1 (next in line)
        → last_processed advances to 1

Step 3: Call skip_canceled_queue_request with request #2
        → last_processed advances to 2
        → tombstone account closed, rent returned to original requester

Step 4: Authority approves request #3
        → last_processed advances to 3 ✓
```

---

## Instructions Summary

### SubscriptionQueue (deposits)

| Instruction                     | Who Can Call                            | What It Does                                                    |
| ------------------------------- | --------------------------------------- | --------------------------------------------------------------- |
| `initialize_subscription_queue` | Vault authority (before initialization) | Adds the FIFO deposit queue extension to the vault              |
| `create_deposit_request`        | Any user                                | Creates a deposit request, assigns it the next queue ID         |
| `approve_request`               | Vault authority                         | Approves the next deposit in queue order                        |
| `reject_request`                | Vault authority                         | Rejects the next deposit in queue order                         |
| `cancel_queued_deposit_request` | Request owner                           | Transfers assets back immediately; leaves tombstone open        |
| `skip_canceled_queue_request`   | Anyone                                  | Advances the queue past a tombstone; closes it and returns rent |

### RedemptionQueue (redemptions)

| Instruction                        | Who Can Call                            | What It Does                                                    |
| ---------------------------------- | --------------------------------------- | --------------------------------------------------------------- |
| `initialize_redemption_queue`      | Vault authority (before initialization) | Adds the FIFO redemption queue extension to the vault           |
| `create_redeem_request`            | Any user                                | Creates a redeem request, assigns it the next queue ID          |
| `approve_request`                  | Vault authority                         | Approves the next redeem in queue order                         |
| `reject_request`                   | Vault authority                         | Rejects the next redeem in queue order                          |
| `cancel_queued_redemption_request` | Request owner                           | Mints burned shares back immediately; leaves tombstone open     |
| `skip_canceled_queue_request`      | Anyone                                  | Advances the queue past a tombstone; closes it and returns rent |
