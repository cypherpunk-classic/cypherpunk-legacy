//! Money nobody can trace: the wallet of Chaumian eCash from the Commons Mint,
//! the plain IOUs you collect before you have one, Hashcash stamps, and what
//! the shops sell.
//!
//! Every payment runs the real protocol: the wallet hands tokens to the shop,
//! the shop deposits them at the mint, the mint checks C = k·hash_to_curve(x)
//! and its spent list, and the change comes back as freshly blinded tokens.

use cpl_crypto::bytes::to_hex;
use cpl_crypto::drbg::Drbg;
use cpl_crypto::ecash::{self, Mint, Token, dleq_verify, finish, prepare, split_amount};
use cpl_crypto::secp::compressed;

use crate::state::{GameState, StateEvent, TokenData};

impl From<&TokenData> for Token {
    fn from(t: &TokenData) -> Self {
        Token { amount: t.amount, secret: t.secret.clone(), c: t.c.clone() }
    }
}

impl From<Token> for TokenData {
    fn from(t: Token) -> Self {
        TokenData { amount: t.amount, secret: t.secret, c: t.c }
    }
}

/// What a payment did, for the receipt a terminal shows.
#[derive(Clone, Debug, Default)]
pub struct Receipt {
    pub paid: u64,
    /// the tokens handed over (amount, short secret)
    pub tokens: Vec<(u64, String)>,
    pub change: u64,
}

/// Something a shop sells.
#[derive(Clone, Copy, Debug)]
pub struct Ware {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub price: u64,
    /// the keyring item it gives, if any
    pub item: Option<&'static str>,
    pub shop: &'static str,
}

pub const WARES: &[Ware] = &[
    Ware { id: "faraday", name: "Faraday pouch", desc: "A copper-lined pouch for your phone. A mast cannot log a device it cannot hear.", price: 6, item: Some("faraday"), shop: "stall" },
    Ware { id: "mesh-radio", name: "Mesh radio", desc: "A pocket radio that relays for its neighbours. Your allies' relays reach you anywhere.", price: 8, item: Some("mesh-radio"), shop: "stall" },
    Ware { id: "coffee", name: "Coffee", desc: "Hot, bitter, paid for in cash nobody can trace.", price: 1, item: None, shop: "vending" },
    Ware { id: "fish", name: "Dried fish", desc: "Nonce's favourite. Grandpa's letter said: once. Only once.", price: 1, item: None, shop: "vending" },
    Ware { id: "print-run", name: "Print run", desc: "Two hundred copies of a book, bound, no questions asked.", price: 5, item: None, shop: "press" },
];

pub fn ware(id: &str) -> Option<&'static Ware> {
    WARES.iter().find(|w| w.id == id)
}

impl GameState {
    /// The wallet is open once you have been to the Commons Mint.
    pub fn wallet_open(&self) -> bool {
        self.flag("wallet.open")
    }

    pub fn credits(&self) -> u64 {
        self.data.wallet.iter().map(|t| t.amount).sum()
    }

    /// The mint as it stands in this save: its keys, and the spent list so far.
    pub fn mint(&self) -> Mint {
        let mut m = self.world().mint();
        for y in &self.data.mint_spent {
            if let Ok(b) = cpl_crypto::bytes::from_hex(y)
                && let Ok(a) = <[u8; 33]>::try_from(b.as_slice())
            {
                m.spent.insert(a);
            }
        }
        m
    }

    fn keep_spent(&mut self, m: &Mint) {
        self.data.mint_spent = m.spent.iter().map(|y| to_hex(y)).collect();
    }

    fn wallet_rng(&mut self) -> Drbg {
        self.data.wallet_nonce += 1;
        self.rng(&format!("wallet/{}", self.data.wallet_nonce))
    }

    /// Withdraw `amount` as blind-signed tokens (wallet and mint run the full
    /// protocol, DLEQ proof included).
    fn withdraw(&mut self, amount: u64) {
        let mut mint = self.mint();
        let mut rng = self.wallet_rng();
        for a in split_amount(amount) {
            let p = prepare(a, &mut rng);
            let (c_blind, proof) = mint.sign(a, &p.b_blind, "", &mut rng).expect("known amount");
            let k_pub = mint.public_key(a).expect("known amount");
            debug_assert!(dleq_verify(&proof, &k_pub, &p.b_blind, &c_blind));
            self.data.wallet.push(finish(&p, &c_blind, &k_pub).into());
        }
    }

    /// Pay the player: eCash if the wallet is open, otherwise an IOU with their name on it.
    pub fn earn(&mut self, amount: u64, why: &str) {
        if amount == 0 {
            return;
        }
        if self.wallet_open() {
            self.withdraw(amount);
        } else {
            self.data.ious += amount;
        }
        self.data.stats.earned += amount;
        self.events.push(StateEvent::Credits(amount as i64, why.to_string()));
    }

    /// Swap all IOUs for eCash. Returns how much was swapped.
    pub fn swap_ious(&mut self) -> u64 {
        let n = self.data.ious;
        if n > 0 {
            self.data.ious = 0;
            self.withdraw(n);
        }
        n
    }

    /// Pay `amount` to `to`: hand over tokens, the payee deposits them at the
    /// mint (which checks each signature and its spent list), and the change
    /// comes back as new blind tokens.
    pub fn pay(&mut self, amount: u64, to: &str) -> Result<Receipt, String> {
        if self.credits() < amount {
            return Err(format!("{amount} cr needed, you have {}", self.credits()));
        }
        let tokens: Vec<Token> = self.data.wallet.iter().map(Token::from).collect();
        let pick = ecash::select(&tokens, amount).ok_or("not enough tokens")?;
        let mut mint = self.mint();
        let mut receipt = Receipt { paid: amount, ..Default::default() };
        let mut handed = 0;
        for &i in &pick {
            mint.redeem(&tokens[i]).map_err(|e| format!("the mint refused a token: {e}"))?;
            handed += tokens[i].amount;
            receipt.tokens.push((tokens[i].amount, tokens[i].secret[..8].to_string()));
        }
        let mut sorted = pick.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for i in sorted {
            self.data.wallet.remove(i);
        }
        self.keep_spent(&mint);
        receipt.change = handed - amount;
        if receipt.change > 0 {
            self.withdraw(receipt.change);
        }
        self.data.stats.spent += amount;
        self.data.stats.payments += 1;
        self.events.push(StateEvent::Credits(-(amount as i64), to.to_string()));
        Ok(receipt)
    }

    /// The Directorate's fine: whatever you can pay of it, IOUs first.
    pub fn fine(&mut self, amount: u64) -> u64 {
        let from_ious = self.data.ious.min(amount);
        self.data.ious -= from_ious;
        let rest = (amount - from_ious).min(self.credits());
        if rest > 0 {
            let _ = self.pay(rest, "the Directorate");
        }
        from_ious + rest
    }

    /// Try to spend a token twice (the mint's spent list catches it).
    pub fn double_spend_check(&self, t: &TokenData) -> Result<(), String> {
        self.mint().check(&Token::from(t)).map_err(|e| e.to_string())
    }

    pub fn add_stamp(&mut self, stamp: String) {
        self.data.stamps.push(stamp);
        self.data.stats.stamps_minted += 1;
    }

    pub fn take_stamp(&mut self) -> Option<String> {
        (!self.data.stamps.is_empty()).then(|| self.data.stamps.remove(0))
    }

    /// Y (hex) of a token: what the mint puts on its spent list.
    pub fn token_y(t: &TokenData) -> String {
        to_hex(&compressed(&Token::from(t).y()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earn_pay_and_change() {
        let mut s = GameState::new();
        s.new_game("t", 0, Some("econ"));
        s.earn(5, "before the wallet");
        assert_eq!((s.data.ious, s.credits()), (5, 0));
        s.set("wallet.open", true);
        assert_eq!(s.swap_ious(), 5);
        s.earn(8, "a job");
        assert_eq!(s.credits(), 13);
        let r = s.pay(6, "the stall").unwrap();
        assert_eq!(s.credits(), 7);
        assert_eq!(r.paid, 6);
        assert_eq!(s.data.mint_spent.len(), r.tokens.len());
        // a token already spent is refused
        let mut mint = s.mint();
        let spent_y: Vec<String> = s.data.mint_spent.clone();
        assert!(!spent_y.is_empty());
        let fresh = s.data.wallet[0].clone();
        assert!(s.double_spend_check(&fresh).is_ok());
        mint.redeem(&Token::from(&fresh)).unwrap();
        assert!(mint.redeem(&Token::from(&fresh)).is_err());
        assert!(s.pay(100, "x").is_err());
        assert_eq!(s.fine(3), 3);
        assert_eq!(s.credits(), 4);
    }
}
