# Vault Standard Suite

# Summary

We aim to create a standard factory program that handles many of the same use cases as ERC 4626 and 7540. This is an important primitive that would make it easier and safer for others to develop on top of. By handling the subscription/redemption process within a standard implementation, we can promote more lindiness of critical Solana infrastructure, while still allowing innovation on top.

## [Glossary](./GLOSSARY.md)

## **Background**

RWA issuers build custom smart contracts and infrastructure to support their tokenization efforts on Solana. Every product is slightly different, but the high level requirements are all very similar. Tokens need KYC checks, products should have role based access control, and investors should be able to subscribe and redeem from the RWA. This led us to discuss what it would take to build core primitives for these components to make it easier and more secure to deploy on Solana. 

Vaults are just one piece of the puzzle. [sRFC 37](https://forum.solana.com/t/srfc-37-efficient-block-allow-list-token-standard/4036), the Token Access Control List (ACL), standardizes a pattern for handling KYC of a Token without compromising composability (i.e. an improvement to Transfer Hooks). These initiatives together enable KYC’d tokens with subscription/redemption capabilities without the need to deploy custom smart contracts.

Institutions and enterprises that want to manage tokenized funds and other assets require functionality akin to vaults. Vaults handle deposits and redemptions into managed strategies, such as depositing stablecoins to receive shares in a fund. As it stands today, Solana has no standardization for vaults and every team has been left to develop their own implementation. This leads to more integration work for clients, more engineering work for those developing the product, and ultimately leads to less secure Solana programs as the surface area for vulnerabilities increases. This is why we are presenting this standardized vault program proposal.

## **Proposal**

A new program that takes inspiration from [ERC4626](https://eips.ethereum.org/EIPS/eip-4626) and other vault standards as well as the lessons from Token2022, with the intent to be highly customizable by supporting the most common use cases available as extensions.

The creation of a Vault does not create a new Mint for the share token, but rather accepts a pre-configured mint as the share token. This decouples the Vault program from future Mint configuration combinations significantly reducing complexity during Vault creation as well as reducing the likelihood of required program upgrades with new Token Extensions in the future.

As a corollary, the program will not initialize token accounts nor enforce ATAs. The user/admin must initialize in an instruction prior to interacting with the vault program. This promotes maximum flexibility for those that want to use non ATAs.

## **Programs:**

### **Async Vault**

The primary vault implementation supporting asynchronous deposit and redemption flows, where requests are queued and settled by a vault authority. This program is intended to be used across a wide variety of applications with the most influence from Real World Asset (RWA) issuers, teams implmenting offchain strategies, and others where regulatory compliance is required.

- [Sequence Diagrams](programs/async_vault/docs/SEQUENCES.md)

### **Vault (Atomic Vault)**

This is a MVP, it's not production ready since we have decided to focus on the Async Vault implementation.

## Feedback

To suggest a feature or modification, please open an issue in the repository with a detailed explanation of what the request feature/change is and the reasoning behind it.

## Notes

These programs are unoptimized and written in Anchor simply for the speed of development. Feedback is welcome and optimizations will be implemented once there is consensus that the structure of the program in question is relatively stable.
