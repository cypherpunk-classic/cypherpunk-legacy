# Cypherpunk Legacy - Project Structure

## Directory Organization

```
/cypherpunk-legacy/
├── index.html                 # Main entry point / game selection menu
├── LICENSE                    # Project license
│
├── docs/                      # Documentation
│   ├── README.md             # Original project README
│   ├── EXPERIMENT_SUMMARY.md # Development notes
│   ├── STRUCTURE.md          # This file - project structure guide
│   └── crypto-modules/       # Cryptographic module documentation
│       └── README.md         # Crypto modules overview
│
├── src/                       # Source code
│   ├── html/                 # HTML game files
│   │   ├── grandpa-room.html       # Main game - Grandpa's Cabin scene
│   │   ├── bip39-game.html         # BIP39 authentication game
│   │   ├── encryption-demo.html    # Encryption demonstration
│   │   └── pixel-art-test.html     # Pixel art testing playground
│   │
│   ├── css/                  # Stylesheets
│   │   ├── grandpa-room.css        # Styles for Grandpa's Cabin
│   │   ├── bip39-game.css          # (Future) BIP39 game styles
│   │   └── encryption-demo.css     # (Future) Encryption demo styles
│   │
│   └── js/                   # JavaScript modules
│       ├── grandpa-room/           # (Future) Grandpa room game modules
│       │   ├── game.js             # Main game loop
│       │   ├── character.js        # Character sprites and movement
│       │   ├── scenes.js           # Scene rendering
│       │   └── bip39-modal.js      # BIP39 authentication modal
│       └── utils/                  # Shared utilities
│           └── crypto-utils.js     # (Future) Crypto helper functions
│
├── assets/                    # Game assets
│   ├── sprites/              # Character and object sprites
│   └── images/               # Background images and textures
│
└── lib/                       # External libraries
    └── (crypto libraries like rust-crypto-wasm will go here)
```

## Current Status

### ✅ Completed
- [x] Modular folder structure created
- [x] Documentation moved to `docs/` folder
- [x] CSS extracted from grandpa-room-prototype.html
- [x] Main index.html entry point created
- [x] HTML files organized in `src/html/`
- [x] External CSS linked in grandpa-room.html
- [x] Crypto modules documentation structure

### 🚧 In Progress
- [ ] Extract JavaScript into modular files
- [ ] Extract CSS from other HTML files (BIP39, encryption demo)
- [ ] Test all functionality after reorganization

### 📋 Planned
- [ ] Create sprite assets directory structure
- [ ] Add crypto library integration (rust-crypto-wasm, etc.)
- [ ] Implement algorithm capture system
- [ ] Add lattice cryptography module
- [ ] Build UC model interactive demonstrations

## File Relationships

### Main Entry Flow
```
index.html (game menu)
    ├─→ src/html/grandpa-room.html (main game)
    │       └─→ src/css/grandpa-room.css
    │
    ├─→ src/html/bip39-game.html (authentication challenge)
    │
    └─→ src/html/encryption-demo.html (crypto workshop)
```

### Future Modular JS Structure
```
src/html/grandpa-room.html
    ├─→ src/css/grandpa-room.css
    └─→ src/js/grandpa-room/
            ├─→ game.js (game loop, canvas setup)
            ├─→ character.js (player sprites, 8-direction movement)
            ├─→ scenes.js (cabin, Cryptopolis rendering)
            ├─→ furniture.js (computer, drawer, bed, bookshelf)
            └─→ bip39-modal.js (authentication modal logic)
```

## Design Principles

### Modularity
- **Separation of Concerns**: HTML (structure), CSS (style), JS (behavior)
- **Reusable Components**: Shared utilities in `/src/js/utils/`
- **Independent Modules**: Each game module can be developed independently

### Scalability
- **Crypto Module System**: Easy to add new cryptographic algorithms
- **Asset Organization**: Clear structure for sprites, images, sounds
- **Library Integration**: Dedicated `lib/` folder for external dependencies

### Educational Focus
- **Progressive Learning**: From basic (BIP39) to advanced (lattice crypto)
- **Documentation First**: Every module has corresponding docs
- **Interactive Examples**: Learn by playing, not just reading

## Adding New Crypto Modules

### Steps:
1. **Create Documentation**
   ```bash
   mkdir docs/crypto-modules/[algorithm-name]
   # Add README.md, theory.md, implementation.md, challenges.md
   ```

2. **Implement Game Module**
   ```bash
   # Create HTML game file
   touch src/html/[algorithm-name]-game.html

   # Create associated CSS
   touch src/css/[algorithm-name].css

   # Create JS modules
   mkdir src/js/[algorithm-name]
   touch src/js/[algorithm-name]/game.js
   ```

3. **Add to Main Menu**
   - Update `index.html` with new game card
   - Link to new HTML file

4. **Document Implementation**
   - Update this STRUCTURE.md
   - Add to crypto-modules/README.md

## Development Workflow

### Local Testing
```bash
# Serve the project locally
python -m http.server 8000

# Or use any static file server
npx http-server .
```

Navigate to `http://localhost:8000/` to see the main menu.

### Code Organization Best Practices
- Keep HTML files focused on structure and content
- Extract all styles to external CSS files
- Modularize JavaScript by functionality
- Document all cryptographic implementations
- Test educational progression flow

## External Libraries

### Planned Integrations

**Rust Crypto WASM**
- Purpose: High-performance cryptography
- Location: `lib/rust-crypto-wasm/`
- Usage: Advanced encryption scenarios

**BIP39 Library**
- Currently: Inline in HTML files
- Planned: Extract to `lib/bip39/`

**Crypto-JS**
- Purpose: General cryptographic functions
- Location: `lib/crypto-js/`
- Usage: Encryption demo, educational examples

### SageMath Integration (Future)
- For lattice cryptanalysis challenges
- Server-side computation with browser interface

## Resources

- **Main documentation**: `/docs/README.md`
- **Crypto modules**: `/docs/crypto-modules/`
- **Experiment notes**: `/docs/EXPERIMENT_SUMMARY.md`

---

Last updated: 2025-11-12
