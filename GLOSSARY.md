# Glossary

**Asset**: The token deposited into the vault by the user (aka investor) in order to receive ownership shares of the vault.

**Share**: Token representing ownership in the vault that is transferred to the user after a successful deposit.

**Async Vault:** A vault where shares are not atomically distributed to a user during a deposit OR assets are not transferred to a user atomically during a withdrawal. The vault authority must update the NAV of the vault before pending deposits/withdrawals are claimable.

**Extension:** Additional data structure that is appended to the vault account, which may include separate conditional logic during core instruction execution.

**Core Instruction**: Instruction available to ALL vaults, though may have conditional logic given the extensions initialized by the Vault.

**Extension Instruction**: Instruction that is available only due to the existence of an extension on the initialized Vault.

## Notes

In the context of Async Vaults, **Subscription** and **Deposit** are used interchangbly. **Redemption** and **Withdrawal** are also used interchangbly.
