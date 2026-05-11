# SubscriptionQueue Extension

## Overview

The `SubscriptionQueue` extension enforces **first-in, first-out (FIFO) ordering** for deposit requests. When this extension is active on a vault, the vault authority must approve or reject deposits in the exact order they were submitted — the oldest pending deposit must always be processed before any newer ones.

This gives depositors a fair, predictable queue position and prevents any single deposit from being skipped or jumped ahead of others.

---

## How It Works

### Counters

The extension stores two numbers on the vault account:

| Field | Purpose |
|---|---|
| `all_time_total_subscription_requests` | Incremented every time a new deposit request is created. Acts as a sequential ticket counter. |
| `last_processed_subscription_request_index` | Tracks the ID of the most recently approved or rejected deposit. The queue only advances forward. |

When a user submits a deposit request, the vault's counter is incremented and the new value is stamped onto the request account as its **ID**. For example, the first deposit ever gets ID `1`, the second gets `2`, and so on.

### Enforcement

When the vault authority tries to approve or reject a deposit, the program checks:

```
request.id == last_processed + 1
```

If the IDs don't match — meaning an earlier deposit is still pending — the transaction fails with `SubscriptionQueueOutOfOrder`. This makes it impossible to process deposits out of turn.

After a deposit is successfully approved or rejected, `last_processed` advances to that request's ID.

---

## Lifecycle of a Deposit Request

```
User submits deposit
        │
        ▼
Request created → assigned ID (counter + 1)
        │
        ├──► Authority approves or rejects (in order) → last_processed advances → account closed
        │
        └──► User cancels → assets refunded immediately → account stays open as tombstone
                    │
                    ▼
             Anyone calls skip_canceled → queue advances → tombstone closed, rent returned
```

---

## Canceling a Queued Deposit

### The Problem

Canceling a deposit is straightforward from the user's perspective — they want their assets back. But in a FIFO queue, a canceled deposit leaves a **gap** in the ID sequence. If that gap is never resolved, the queue gets permanently stuck: the next deposit has ID `N+2`, but the queue is still waiting for `N+1` which no longer exists.

A naive fix, closing the account immediately and marking the queue as advanced, creates a different problem: a malicious user could cancel deposits at will to stall the queue, or worse, loop through many rapid create-and-cancel cycles to overflow a fixed-size tracking structure. This makes the queue permanently unrecoverable at negligible cost.

### The Solution: Tombstone Accounts

When a user cancels a queued deposit:

1. **Assets are refunded immediately** — the user gets their money back right away.
2. **The request account stays open** in a `Canceled` state rather than being closed.

The open account acts as a **tombstone** — a lightweight marker that proves this ID existed and was properly canceled. No queue state is modified yet; the tombstone just sits there until the queue naturally reaches it.

Because accounts cost rent (a small ongoing fee paid in SOL), keeping them open is not free. This prevents spam: an attacker trying to create many tombstones to grief the queue must pay rent for every open account they leave behind, and the queue can always be unblocked.

### Why Not Use `cancel_request`?

The existing `cancel_request` instruction closes the account entirely when it finishes. Using it on a queued deposit would destroy the tombstone, recreating the stuck-queue bug. To prevent this, `cancel_request` now returns an error (`MustUseCancelQueuedDepositRequest`) if called on a deposit that belongs to a subscription-queue vault. Callers should use `cancel_queued_deposit_request` instead.

Redeems and deposits on non-queued vaults are unaffected — `cancel_request` still closes their accounts immediately as before.

---

## Advancing Past a Tombstone

### `skip_canceled_subscription_request`

Once the queue has processed all earlier deposits and arrives at a tombstone, someone must tell the program to skip over it. This is done by calling `skip_canceled_subscription_request`.

**This instruction is permissionless** — anyone can call it, not just the vault authority or the original depositor. This is intentional: if the vault authority is slow to act, the depositor (or anyone else) can unblock the queue themselves.

When called, the instruction:

1. Verifies the request is in `Canceled` state and is the next expected ID (`last_processed + 1`).
2. Advances `last_processed` to this request's ID.
3. Closes the tombstone account and **returns the rent to the original depositor**.

Multiple consecutive tombstones must each be skipped with a separate call, in order. Each call advances the queue by one position.

### Example: Cancel in the Middle of a Queue

Suppose three deposits exist with IDs 1, 2, and 3, and deposit 2 is canceled:

```
Step 1: Cancel deposit #2
        → assets refunded, account stays open (tombstone)
        → last_processed = 0

Step 2: Authority approves deposit #1 (next in line)
        → last_processed advances to 1

Step 3: Call skip_canceled_subscription_request with deposit #2
        → last_processed advances to 2
        → tombstone account closed, rent returned to original depositor

Step 4: Authority approves deposit #3
        → last_processed advances to 3 ✓
```

---

## Instructions Summary

| Instruction | Who Can Call | What It Does |
|---|---|---|
| `initialize_subscription_queue` | Vault authority (before initialization) | Adds the FIFO queue extension to the vault |
| `create_deposit_request` | Any user | Creates a deposit, assigns it the next queue ID |
| `approve_request` | Vault authority | Approves the next deposit in queue order |
| `reject_request` | Vault authority | Rejects the next deposit in queue order |
| `cancel_queued_deposit_request` | Request owner | Refunds assets immediately; leaves tombstone open |
| `skip_canceled_subscription_request` | Anyone | Advances the queue past a tombstone; closes it and returns rent |
