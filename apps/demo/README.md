# Vault Standard Suite — Demo App

A Next.js 15 dapp that exercises every entrypoint of the [`async_vault`](../../programs/async_vault) program. Built for screen-recording: a clean landing page, a 3-step create-vault wizard with all eight extensions, and dual Authority / User consoles for the full deposit → approve → claim and redeem lifecycles.

```
apps/demo
├── src/app/                 — App Router pages (/, /create, /vaults, /vault/[shareMint])
├── src/components/          — UI primitives + vault-specific components
├── src/lib/program.ts       — Wraps the codama-generated @solana/vault client into web3.js TransactionInstructions
├── src/lib/extensions.ts    — Reads the TLV extension region from raw vault account data
└── src/lib/hooks/use-vault.ts — Fetches the vault, mints, balances, and program-owned Request accounts
```

## Stack

- Next.js 15 App Router, React 19, Tailwind v3, lucide-react, sonner toasts
- `@solana/wallet-adapter-react(-ui)` for wallet connection (Wallet Standard auto-detects Phantom, Backpack, Solflare, etc.)
- `@solana/web3.js` v1 for transaction sending; signing handled by the wallet adapter
- `@solana/kit` instruction objects (from the workspace `@solana/vault` codama client) bridged into web3.js via `src/lib/kit-bridge.ts`

## Local development

From the repo root:

```bash
pnpm install               # workspace install (installs the demo too)
pnpm run generate-clients  # only needed if idl/ has changed since last generation
pnpm --filter @vault-suite/demo dev
```

Open http://localhost:3000.

The demo defaults to devnet. Set environment variables in `apps/demo/.env.local` to override:

```bash
NEXT_PUBLIC_CLUSTER=devnet
NEXT_PUBLIC_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_PROGRAM_ID=7M6pdteAnZmj9SEyzjsqUEqfcc4jqhpgLFF9dULDq1iP
```

## Deploying to Vercel

This is a pnpm monorepo, so the deploy needs to run install at the repo root. Two options:

### Option A — point Vercel at the repo

1. Push the repo to GitHub.
2. In Vercel, "Import Project" → pick the repo.
3. Set the **Root Directory** to `apps/demo`.
4. Vercel auto-detects Next.js. The `apps/demo/vercel.json` shipped here already runs `pnpm install` at the repo root before building.
5. Add the three environment variables above (use a real RPC like Helius or Triton — the public devnet RPC is heavily rate-limited).

### Option B — `vercel` CLI from `apps/demo`

```bash
cd apps/demo
vercel --prod
```

Either way, the program must already be deployed on the cluster you point at. **The default program ID is the devnet/localnet build of `async_vault`.** If you've redeployed the program with `anchor deploy`, set `NEXT_PUBLIC_PROGRAM_ID` to your new program address.

## Deploying the program to devnet

```bash
# from repo root
solana config set --url devnet
solana airdrop 4

anchor build
solana program deploy target/deploy/async_vault.so

# Then:
#   1. Update declare_id! in programs/async_vault/src/lib.rs and Anchor.toml
#   2. Re-run `pnpm run generate-clients`
#   3. Redeploy
#   4. Set NEXT_PUBLIC_PROGRAM_ID accordingly
```

## How the demo is wired

Most of the interesting logic lives in `src/lib/program.ts`. Each helper takes plain web3.js `PublicKey`s, builds the kit `Instruction` via the codama client, and converts to a `TransactionInstruction` so wallet-adapter can sign and submit it.

For account state we read raw bytes via `connection.getAccountInfo` and decode with the codama-generated `getVaultDecoder()` / `getRequestDecoder()`. The vault's TLV extension region is parsed by hand in `src/lib/extensions.ts` — Codama doesn't model it because it's appended past the fixed-size struct.

Pending request lookup uses `getProgramAccounts` with two memcmp filters:

1. byte 0 — Request discriminator
2. byte 8 — `vault` field (offset right after the discriminator)

## 90-second video script

This is the path the landing page hints at; it touches every interesting feature.

1. **Connect Phantom** (top right). The cluster pill confirms you&apos;re on devnet.
2. **Create vault** → step 1: pick "Synthetic demo asset", leave the defaults.
3. → step 2: toggle on **Deposit Fee** (50 bps) and **Pausable Subscriptions**, then **Min Subscription** with threshold `5`.
4. → step 3: hit **Deploy vault**. The wizard sends two transactions: one to mint the demo asset, one to deploy the vault, init the three extensions, and finalize.
5. You land on the vault page. Show the **Overview** tab with the extensions and stats.
6. Switch to **User portal**. Try a deposit of `2` (under the threshold) — it should fail. Then `100` — request appears in **Your active requests** as **Pending**.
7. Switch to **Authority** tab. Hit **Update NAV** with `1.05`. Then **Approve** the pending request. The deposit fee is taken; the rest of the assets land in the reserve.
8. Switch back to **User portal**, click **Claim**. Shares are minted to your wallet.
9. Submit a **Redeem** request, switch to authority, **Approve**. Switch back, **Claim** to receive assets.
10. Bonus: Authority tab → toggle **Pause subscriptions**, try to deposit again from the user tab to show the pause is enforced.

## Caveats

- The `set_operator` instruction requires the operator to sign too, which is awkward in a single-wallet demo. The User portal "Set operator" button generates a fresh ephemeral keypair, signs with it, and surfaces the resulting pubkey via a toast so you can show it on camera.
- The vault account's TLV region is parsed manually — if you change extension layouts in the program, update `src/lib/extensions.ts` accordingly.
- This demo deliberately uses `wallet-adapter` legacy `Connection` for signing/sending. Instruction building goes through `@solana/kit` + the codama client, then the kit-bridge converts to web3.js. See `src/lib/kit-bridge.ts`.
