# Crypto Modules Documentation

This directory contains documentation for cryptographic modules and algorithms implemented in Cypherpunk Legacy.

## Planned Modules

### 1. Algorithm Capture System (UC Model)
**Status**: Planned

The Universal Composability (UC) model allows for the composition of cryptographic protocols in a provably secure way.

**Planned Features**:
- Collect individual encryption algorithms as "capturable" entities
- Combine algorithms to create more complex cryptographic systems
- Visual representation of protocol composition
- Interactive learning about security under composition

**Resources**:
- [UC Model Overview](https://en.wikipedia.org/wiki/Universal_composability)
- [PCS ZKPunk](https://pcs.zkpunk.pro/)

### 2. Lattice Cryptography & LLL Algorithm
**Status**: Planned

Lattice-based cryptography is a foundation for post-quantum cryptographic systems.

**Planned Features**:
- Interactive lattice visualization
- LLL (Lenstra–Lenstra–Lovász) algorithm demonstrations
- Cryptanalysis challenges using lattice reduction
- Coordinate location puzzles (inspired by Minecraft mod techniques)

**Resources**:
- [Lattice Cryptanalysis - Lacom Wiki](https://wiki.lacom.io/wiki/cryptanysis/cryptanysis-sage-101/)
- [SageMath for Cryptanalysis](https://www.sagemath.org/)

### 3. Modern Crypto Libraries

**Rust Crypto WASM**:
- [Buttercup Rust Crypto WASM](https://github.com/buttercup/rust-crypto-wasm)
- High-performance cryptography compiled to WebAssembly
- Planned integration for advanced encryption scenarios

### 4. Advanced Protocols

Gradually introducing:
- Zero-knowledge proofs
- Homomorphic encryption basics
- Multi-party computation concepts
- Blockchain cryptography fundamentals

## Module Structure Template

Each crypto module should follow this structure:

```
docs/crypto-modules/
  └── [algorithm-name]/
      ├── README.md          # Overview and learning objectives
      ├── theory.md          # Mathematical foundations
      ├── implementation.md  # Implementation details
      ├── challenges.md      # Interactive challenges and puzzles
      └── resources.md       # External resources and references
```

## Contributing

When adding a new cryptographic module:

1. Create a new directory under `crypto-modules/`
2. Follow the template structure above
3. Include clear learning objectives
4. Provide interactive examples where possible
5. Link to authoritative resources
6. Consider the educational progression from basic to advanced concepts

## Philosophy

**Crypto Wars Ideology + Algorithm Education**

This project aims to teach cryptography through the lens of the crypto wars - the historical and ongoing struggle for strong encryption and digital privacy rights. By gamifying cryptographic concepts, we make complex mathematics accessible while emphasizing the importance of cryptography in protecting individual freedom.

### Key Principles:

1. **Learn by Doing**: Interactive gameplay over passive reading
2. **Progressive Complexity**: Start simple, build to advanced topics
3. **Historical Context**: Understand why cryptography matters
4. **Hands-on Implementation**: See algorithms in action
5. **Security Awareness**: Understand both construction and cryptanalysis

## Resources

- [PCS ZKPunk](https://pcs.zkpunk.pro/) - Privacy Computing System with ZK proofs
- [Lacom Wiki](https://wiki.lacom.io/) - Comprehensive cryptography wiki
- [Cypherpunks Mailing List Archive](https://www.activism.net/cypherpunk/) - Historical context

---

*"Privacy is necessary for an open society in the electronic age."* - A Cypherpunk's Manifesto, Eric Hughes, 1993
