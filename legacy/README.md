# 🔐 Cypherpunk Legacy

> *An educational game journey through cryptography, the crypto wars, and the fight for digital privacy*

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-Prototype-yellow.svg)]()

## 🎮 Overview

**Cypherpunk Legacy** is an interactive educational game that teaches cryptography through gameplay. Inspired by the cypherpunk movement and the ongoing crypto wars, this project combines:

- 🎓 **Cryptographic Education** - Learn real encryption algorithms
- 🕹️ **Engaging Gameplay** - Interactive challenges and puzzles
- 📚 **Historical Context** - Understand the crypto wars and why encryption matters
- 🔬 **Hands-on Implementation** - See algorithms in action

### Current Features

✅ **Grandpa's Cabin** - Explore a pixel-art cabin and discover a cryptographic legacy
✅ **BIP39 Authentication** - Learn mnemonic phrases and key derivation (6-word game mode)
✅ **Encryption Demo** - Interactive symmetric/asymmetric cryptography workshop
✅ **8-Direction Movement** - Smooth character controls with diagonal movement
✅ **Z-Ordering System** - Depth-based rendering for object occlusion

## 🚀 Quick Start

### Play Online
Simply open `index.html` in a modern web browser to access the game selection menu.

### Local Development
```bash
# Clone the repository
git clone https://github.com/cypherpunk-classic/cypherpunk-legacy.git
cd cypherpunk-legacy

# Serve locally (choose one)
python -m http.server 8000
# or
npx http-server .

# Open browser to http://localhost:8000
```

## 📁 Project Structure

```
/cypherpunk-legacy/
├── index.html              # 🎯 Main game menu
├── docs/                   # 📚 Documentation
│   ├── README.md
│   ├── STRUCTURE.md        # Detailed structure guide
│   └── crypto-modules/     # Crypto algorithm docs
├── src/
│   ├── html/              # 🎮 Game files
│   │   ├── grandpa-room.html
│   │   ├── bip39-game.html
│   │   └── encryption-demo.html
│   ├── css/               # 🎨 Stylesheets
│   │   └── grandpa-room.css
│   └── js/                # 💻 JavaScript modules (future)
├── assets/                # 🖼️ Game assets
│   ├── sprites/
│   └── images/
└── lib/                   # 📦 External libraries
```

See [docs/STRUCTURE.md](docs/STRUCTURE.md) for detailed documentation.

## 🎯 Roadmap

### Phase 1: Core Gameplay ✅ (Current)
- [x] Pixel art cabin environment
- [x] Character movement and animations (8 directions)
- [x] BIP39 mnemonic authentication system
- [x] Basic encryption demonstrations
- [x] Scene transitions (Cabin → BIP39 → Cryptopolis)

### Phase 2: Algorithm Capture System 🚧 (Next)
- [ ] Collectible encryption algorithm system
- [ ] Algorithm combination mechanics (UC Model)
- [ ] Visual protocol composition
- [ ] Cryptographic puzzle challenges

### Phase 3: Advanced Cryptography 📋 (Planned)
- [ ] **Lattice Cryptography** - LLL algorithm, post-quantum crypto
- [ ] **Cryptanalysis Challenges** - Break weak encryption
- [ ] **Zero-Knowledge Proofs** - Interactive ZK demonstrations
- [ ] **Multi-Party Computation** - Collaborative crypto puzzles

### Phase 4: Library Integration 📋 (Planned)
- [ ] [Rust Crypto WASM](https://github.com/buttercup/rust-crypto-wasm) integration
- [ ] [SageMath](https://www.sagemath.org/) for lattice cryptanalysis
- [ ] Advanced ZK libraries from [PCS ZKPunk](https://pcs.zkpunk.pro/)
- [ ] Educational content from [Lacom Wiki](https://wiki.lacom.io/)

## 🧩 Game Modules

### 🏠 Grandpa's Cabin
Your grandfather, a cypherpunk pioneer, has left you his cabin. Inside, you'll find:
- An old CRT computer requiring BIP39 authentication
- A mysterious letter with mnemonic clues
- A portal to Cryptopolis, the digital city he helped build

**Learning Objectives**: Introduction to cryptographic legacy, BIP39 mnemonics

### 🔑 BIP39 Authentication
Master the art of mnemonic phrases:
- Select from 3 pre-generated phrases or enter your own
- Understand key derivation (PBKDF2)
- See how 6 words can secure digital identity
- Challenge-response authentication flow

**Learning Objectives**: BIP39 standard, key derivation, authentication protocols

### 🔐 Encryption Workshop
Hands-on cryptography:
- Symmetric encryption (AES)
- Asymmetric encryption (RSA)
- Key generation and management
- Encrypt/decrypt messages interactively

**Learning Objectives**: Symmetric vs asymmetric crypto, practical encryption use

## 🎓 Educational Philosophy

### The Crypto Wars
This game teaches cryptography through the lens of the **crypto wars** - the historical and ongoing battle for:
- Strong encryption without backdoors
- Individual privacy rights
- Freedom from mass surveillance
- Open-source cryptographic tools

### Learn by Doing
Instead of passive reading, players:
1. **Explore** cryptographic environments
2. **Experiment** with real algorithms
3. **Solve** puzzles requiring crypto knowledge
4. **Build** complex systems from simple components
5. **Understand** why cryptography matters for freedom

## 🔬 Technical Details

### Technologies
- **Frontend**: Pure HTML5, CSS3, JavaScript (no framework dependencies)
- **Graphics**: Canvas API with pixel-perfect rendering
- **Crypto**: Native Web Crypto API + BIP39 library
- **Future**: Rust-WASM for performance-critical crypto

### Architecture Principles
- **Modular Design**: Separation of HTML, CSS, JS
- **Progressive Enhancement**: Works without modern features
- **Educational Focus**: Code readability over optimization
- **Open Source**: All algorithms are transparent and auditable

## 🤝 Contributing

We welcome contributions! Areas where you can help:

- 🎮 **Game Design** - New puzzles, mechanics, storylines
- 🔐 **Cryptography** - Implementing new algorithms, educational content
- 🎨 **Art** - Pixel art sprites, backgrounds, UI elements
- 📚 **Documentation** - Tutorials, algorithm explanations
- 🐛 **Bug Reports** - Testing and issue reporting

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines (coming soon).

## 📚 Resources

### Learning Materials
- [Crypto Module Documentation](docs/crypto-modules/README.md)
- [PCS ZKPunk](https://pcs.zkpunk.pro/) - ZK proof systems
- [Lacom Wiki](https://wiki.lacom.io/) - Comprehensive cryptography wiki
- [Crypto 101](https://www.crypto101.io/) - Introductory cryptography book

### Historical Context
- [A Cypherpunk's Manifesto](https://www.activism.net/cypherpunk/manifesto.html) (1993)
- [Cypherpunks Mailing List Archive](https://www.activism.net/cypherpunk/)
- [Crypto Wars](https://en.wikipedia.org/wiki/Crypto_Wars) (Wikipedia)

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- The cypherpunk movement and pioneers who fought for encryption
- The open-source cryptography community
- Everyone building tools for digital privacy and freedom

---

*"Privacy is necessary for an open society in the electronic age."*
— Eric Hughes, A Cypherpunk's Manifesto (1993)

**Built with ❤️ for education, privacy, and freedom**
