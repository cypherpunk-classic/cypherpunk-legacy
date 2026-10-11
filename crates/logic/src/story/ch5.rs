//! Chapter 5 · Knowledge. Lin, the checkpoint, zero knowledge, the extractor.

use crate::puzzles::{ExtractorFs, SigmaCheckpoint, SimulatorGame};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

async fn run_sigma(c: &Ctx) {
    if !c.puzzle(Box::new(SigmaCheckpoint::new(c.world()))).await {
        c.objective("Pass the checkpoint without revealing your key.");
        return;
    }
    c.set("ch5.sigma");
    c.codex(&["sigma"]);
    say!(
        c,
        (Lin, "You passed. And the checkpoint still has no idea what your key is. That is a zero-knowledge proof: you proved you know x such that X = x·G, and leaked nothing."),
        (Lin, "Goldwasser, Micali and Rackoff, 1985. The whole field grows out of it. But I can see you do not quite believe the \"leaked nothing\" part. Nobody does at first. Let me prove it to you."),
    )
    .await;
    c.objective("Play Lin's real-or-simulated game.");
}

async fn run_sim(c: &Ctx) {
    if !c.puzzle(Box::new(SimulatorGame::new(c.world()))).await {
        c.objective("Tell the real transcripts from the simulated ones (you cannot).");
        return;
    }
    c.set("ch5.sim");
    say!(
        c,
        (Lin, "Fifty-fifty. Every time. Because I can forge a transcript with no secret that looks exactly like a real one. If a transcript revealed anything about x, you could have used it to beat the game. You could not."),
        (Lin, "Now the sharp edge. The same fact that makes the proof sound, two answers reveal the secret, is a weapon if a prover is careless with its commitment. And the Directorate's checkpoint master is very careless."),
    )
    .await;
    c.objective("Turn the checkpoint's own proof against it (Lin's terminal).");
}

async fn run_extract(c: &Ctx) {
    if !c.puzzle(Box::new(ExtractorFs::new(c.world()))).await {
        c.objective("Extract the checkpoint key and forge a clearance proof.");
        return;
    }
    c.set("ch5.extract");
    c.codex(&["fiat-shamir", "weak-fs"]);
    c.give("checkpoint-key");
}

pub async fn checkpoint(c: Ctx) {
    if !c.flag("ch5.met") {
        say!(
            c,
            (Agent, "Halt. Present your key for inspection. Routine. Everyone does it."),
            (Nonce, "Do not hand that over. Talk to Lin first. The one in the mask. Obviously."),
        )
        .await;
        return;
    }
    if !c.flag("ch5.sigma") {
        return run_sigma(&c).await;
    }
    if c.flag("ch5.passed") {
        c.say1(Sp::Narrator, "The checkpoint scanner flickers and waves you through. It no longer asks for your name.").await;
        return;
    }
    c.say1(Sp::Lin, "You can clear it properly once we are done. Come here first.").await;
}

pub async fn agent(c: Ctx) {
    if !c.flag("ch5.extract") {
        say!(
            c,
            (Agent, "Present your key. It is for your safety. We log it, we protect it, we would never misuse it."),
            (Nonce, "*(quietly)* They lost the last three key logs to breaches. Do not."),
        )
        .await;
        return;
    }
    c.say1(Sp::Agent, "*(the agent squints at your proof, finds it valid, and cannot explain why it feels wrong)* …cleared. Move along.").await;
}

pub async fn lin(c: Ctx) {
    if !c.flag("ch5.met") {
        c.set("ch5.met");
        say!(
            c,
            (Lin, "New face. You have the look of someone the Directorate wants a name for. I'm Lin. I make proofs that convince without confessing."),
            (Lin, "That checkpoint wants your key \"for inspection\". Hand it over and they can impersonate you forever. But you do not have to refuse and you do not have to comply. There is a third option."),
            (Lin, "You prove you hold the key, without showing it. The checkpoint ends up certain, and empty-handed. Watch. Then you try."),
        )
        .await;
        run_sigma(&c).await;
        return;
    }
    if !c.flag("ch5.sigma") {
        return run_sigma(&c).await;
    }
    if !c.flag("ch5.sim") {
        return run_sim(&c).await;
    }
    if !c.flag("ch5.extract") {
        return run_extract(&c).await;
    }
    if !c.flag("ch5.done") {
        c.set("ch5.done");
        c.set("ch5.passed");
        c.chapter(6);
        c.sfx("unlock");
        say!(
            c,
            (Lin, "You extracted their master key by rewinding a sloppy prover, then wrapped it in Fiat–Shamir so you never have to interact with a checkpoint again. And you saw why the hash must bind the context, or every clearance works everywhere."),
            (Lin, "That last bug is not a toy. In 2022 a whole family of real proof systems shipped with weak Fiat–Shamir. \"Frozen Heart\", they called it."),
            (Lin, "The checkpoints are yours now. Walk where you like; tell them nothing."),
            (Lin, "One person left you should meet. Ada, at the Observatory. She does not worry about today's Directorate. She worries about the computer that breaks all of this at once. The gate is open."),
        )
        .await;
        c.objective("Find Ada at the Observatory (north of the plaza).");
        return;
    }
    c.say1(Sp::Lin, "Give Ada my regards. And tell her I still think the quantum computers are further off than she says. She will disagree. She is usually right.").await;
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch5.met") {
        c.say1(Sp::System, "MASQUERADE TERMINAL\nIdentity services. Find Lin.").await;
        return;
    }
    if !c.flag("ch5.sigma") {
        return run_sigma(&c).await;
    }
    if !c.flag("ch5.sim") {
        return run_sim(&c).await;
    }
    if !c.flag("ch5.extract") {
        return run_extract(&c).await;
    }
    c.say1(Sp::System, "MASQUERADE TERMINAL\nClearance proofs: active. Checkpoints: bypassed.").await;
}
