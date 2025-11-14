// BIP39 Module Import
import * as bip39Module from 'https://esm.sh/bip39@3.1.0';
window.bip39 = bip39Module;

// Crypto utilities
window.crypto_utils = {
    simpleSign: async function(privateKey, message) {
        const encoder = new TextEncoder();
        const data = encoder.encode(privateKey + message);
        const hashBuffer = await crypto.subtle.digest('SHA-256', data);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
    },

    simpleVerify: async function(privateKey, message, signature) {
        const expectedSignature = await this.simpleSign(privateKey, message);
        return expectedSignature === signature;
    }
};

// System ready
window.systemReady = true;
const statusEl = document.getElementById('systemStatus');
if (statusEl) {
    statusEl.textContent = 'AWAITING AUTHENTICATION';
    statusEl.style.color = '#9a9a9a';
}

console.log('Cryptopolis Access Terminal initialized');
