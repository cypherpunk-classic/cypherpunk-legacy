//! Chapter 6 · After the quantum. Ada, lattices, the knapsack, LWE and ML-KEM.

use crate::puzzles::{GaussReduce, KnapsackBreak, LweMlkem};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

async fn run_gauss(c: &Ctx) {
    if !c.puzzle(Box::new(GaussReduce::new(c.world()))).await {
        c.objective("Reduce the lattice basis at Ada's workshop.");
        return;
    }
    c.set("ch6.gauss");
    c.codex(&["lattice", "shor"]);
    say!(
        c,
        (Ada, "Good. That pairwise reduce-and-swap, run across many dimensions, is LLL: Lenstra, Lenstra and Lovász, 1982. Polynomial time, and it finds surprisingly short vectors."),
        (Ada, "Lattices are the double edge of the next era. They break old schemes that hid structure badly, and they build new ones that hide it well. Let me show you the breaking first. The Directorate will hate it."),
    )
    .await;
    c.objective("Break the Directorate's QSAFE channel (Ada's terminal).");
}

async fn run_knapsack(c: &Ctx) {
    if !c.puzzle(Box::new(KnapsackBreak::new(c.world()))).await {
        c.objective("Break the QSAFE knapsack with LLL.");
        return;
    }
    c.set("ch6.knapsack");
    c.codex(&["knapsack"]);
    say!(
        c,
        (Ada, "Their \"quantum-safe\" channel was a 1978 knapsack with a 2024 logo. LLL peeled it open. Hiding an easy problem inside a hard-looking one only works until someone brings the right lattice."),
        (Ada, "But do not walk away thinking lattices are just a weapon. The real post-quantum schemes are built on lattice problems nobody knows how to break, classically or quantumly. Here. Build one instead of breaking one."),
    )
    .await;
    c.objective("Build a post-quantum key (Ada's terminal).");
}

async fn run_pq(c: &Ctx) {
    if !c.puzzle(Box::new(LweMlkem::new(c.world()))).await {
        c.objective("Understand LWE and run an ML-KEM encapsulation.");
        return;
    }
    c.set("ch6.pq");
    c.codex(&["lwe", "mlkem", "nist-pqc"]);
    c.give("pq-keys");
}

pub async fn ada(c: Ctx) {
    if !c.flag("ch6.met") {
        c.set("ch6.met");
        say!(
            c,
            (Ada, "You made it past the checkpoints without leaving a name. Lin's work. I'm Ada. I was the youngest in your grandfather's circle, and now I am the one who stays up worrying about the end of all of it."),
            (Ada, "Everything you have done tonight, every signature, every key exchange, rests on two problems being hard: factoring, and discrete logs. A large enough quantum computer solves both in an afternoon. Shor, 1994."),
            (Ada, "It does not exist yet. But the Directorate is recording everything now, to decrypt later. Harvest now, decrypt later. So we move the ground before the flood. Start with the shape of the new ground: a lattice."),
        )
        .await;
        run_gauss(&c).await;
        return;
    }
    if !c.flag("ch6.gauss") {
        return run_gauss(&c).await;
    }
    if !c.flag("ch6.knapsack") {
        return run_knapsack(&c).await;
    }
    if !c.flag("ch6.pq") {
        return run_pq(&c).await;
    }
    if !c.flag("ch6.done") {
        c.set("ch6.done");
        c.chapter(6);
        c.sfx("unlock");
        say!(
            c,
            (Ada, "ML-KEM. FIPS 203, standardised in 2024, from a scheme called Kyber. Bigger keys than a curve, and it does not fall to Shor. The browsers already use it. Signal already uses it. The flood is coming and the arks are being built in the open, where we can all check them."),
            (Ada, "That is the whole thing, really. Your grandfather's whole bet. Not any one algorithm, those come and go, but doing it in public, so the next people can check our work and build on it."),
            (Ada, "You have the Archive, the keys, and now the map of what comes next. There is only one thing left to do with all of it."),
            (Nonce, "He means the Vault. The real one. Go home, {{handle}}. Finish it."),
        )
        .await;
        c.objective("Return to the Vault on Legacy Plaza to finish what Grandpa started.");
        c.set("end.ready");
        return;
    }
    c.say1(Sp::Ada, "Go to the plaza. We will all be there. We have been waiting a long time for someone to finish his sentence.").await;
}

pub async fn telescope(c: Ctx) {
    say!(
        c,
        (Narrator, "Through the telescope: not stars, but the lattice of the city itself, points of light on an infinite grid, stretching past the horizon in every direction."),
        (Ada, "Every secure thing is a hard question about one of those grids. Pretty, isn't it."),
    )
    .await;
}

pub async fn whiteboard(c: Ctx) {
    c.say1(Sp::Narrator, "A whiteboard dense with Ada's handwriting: \"SVP · CVP · LWE · SIS · NTRU · Kyber=ML-KEM · Dilithium=ML-DSA · SPHINCS+=SLH-DSA\". In the corner: \"SIKE — broken 2022, one laptop, one hour. Do it in the open.\"").await;
    c.codex(&["nist-pqc"]);
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch6.met") {
        c.say1(Sp::System, "OBSERVATORY TERMINAL\nPost-quantum research. Speak with Ada.").await;
        return;
    }
    if !c.flag("ch6.gauss") {
        return run_gauss(&c).await;
    }
    if !c.flag("ch6.knapsack") {
        return run_knapsack(&c).await;
    }
    if !c.flag("ch6.pq") {
        return run_pq(&c).await;
    }
    c.say1(Sp::System, "OBSERVATORY TERMINAL\nMigration complete. The ground has moved.").await;
}
