# fuelname-contracts

Fuelname is a decentralized domain name system for the [Fuel blockchain](https://fuel.network/), inspired by
the [Ethereum Name Service (ENS)](https://ens.domains/). It allows users to register human-readable names, map them to
blockchain addresses or metadata, and manage their domains seamlessly. This repository contains three main smart
contracts: `Registry`, `Resolver`, and `Registrar`.

## Overview

### Features

- **Decentralized Domain Management**: Domains are represented as NFTs (native assets on Fuel), allowing full control
  and ownership by the domain holders.
- **Upgradeable Contracts**: All contracts comply with
  the [SRC-14](https://docs.fuel.network/docs/sway-standards/src-14-simple-upgradeable-proxies/) standard for
  upgradability, enabling the addition of new features over time.
- **Interoperability**: Implements [SRC-7](https://docs.fuel.network/docs/sway-standards/src-7-asset-metadata/) (Asset
  Metadata) and [SRC-20](https://docs.fuel.network/docs/sway-standards/src-20-native-asset/) (Native Asset) standards,
  ensuring compatibility within the Fuel ecosystem.
- **Extensible Metadata Mapping**: Supports custom resolvers for mapping domains to various metadata, including
  blockchain addresses, social media handles, and more.
- **Expiration and Renewal**: Domains have an expiration date, with a grace period to protect ownership.
- **Generational Tracking**: Expired domains that are not renewed after the grace period are re-minted as new assets
  with updated metadata for the next generation.

---

## Smart Contracts

### 1. **Registry**

The `Registry` contract is the core of the Fuelname system. It is responsible for minting domains, managing their
metadata, and lifecycle.

#### Key Features

- **Domain Ownership**: Domains are minted as NFTs. To prove ownership of a domain, the user must add this NFT as an
  input to the transaction (effectively sending it to themselves).
- **Domain Management**: Owners can manage their domains and register subdomains.
- **Expiration Handling**: Domains have an expiration date:
    - **Active Period**: Owners can use their domains as long as they are not expired.
    - **Grace Period**: After expiration, there is a grace period during which ownership is retained, and the owner can
      renew the domain.
    - **Post-Grace Period**: If the grace period expires without renewal, the domain is considered fully expired. A new
      asset can be minted for subsequent registrations.

#### Functions

- **register_high_level_domain**: Registers a new high-level domain (e.g., `fuel`).
- **register_sub_domain**: Registers a subdomain of a given domain (e.g., `mydomain.fuel`).
- **set_resolver**: Sets a resolver for the domain.
- **renew_domain**: Updates the domain expiration timestamp.
- **set_primary**: Sets the domain as primary (enabling reverse resolution from a Fuel address to the domain).

---

### 2. **Resolver**

The `Resolver` contract maps domains to metadata, making it possible to associate human-readable names with addresses or
other information.

#### Key Features

- **Fuel Address Mapping**: Maps domains to Fuel blockchain addresses (mandatory for all resolvers).
- **Extensible Design**: Supports new resolvers for mapping domains to other metadata like:
    - Social media handles
    - Ethereum or Bitcoin addresses
    - IPFS or centralized storage links

#### Default Resolver

- Supports basic address mappings to Fuel addresses.

#### Custom Resolvers

- Developers can create and add custom resolvers to extend functionality.

---

### 3. **Registrar**

The `Registrar` contract manages the sale and registration of new domains under the `.fuel` high-level domain.

#### Key Features

- **Domain Registration**: Facilitates the purchase of new `.fuel` domains.
- **Price Management**: Supports dynamic pricing for domain registration.
- **Renewals**: Allows users to renew domains to extend ownership.

## Deployment

- create `.env` file in `deploy` directory (see `.env.example`)
- run the following command
```bash
cd deploy && cargo run deploy
```
