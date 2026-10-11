//! Who can talk, how their name looks, and whose portrait sits next to them.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sp {
    You,
    Grandpa,
    Nonce,
    Mara,
    Felix,
    Juno,
    Ibrahim,
    Lin,
    Ada,
    Warden,
    Agent,
    System,
    Narrator,
    Letter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Normal,
    Terminal,
    Narration,
    Letter,
}

/// What the dialogue box needs to know about a speaker.
#[derive(Clone, Debug)]
pub struct SpeakerInfo {
    pub name: String,
    pub color: &'static str,
    pub style: Style,
    /// Character sheet id for the portrait ("cat" for Nonce), if any.
    pub portrait: Option<String>,
}

impl Sp {
    pub fn id(self) -> &'static str {
        match self {
            Sp::You => "you",
            Sp::Grandpa => "grandpa",
            Sp::Nonce => "nonce",
            Sp::Mara => "mara",
            Sp::Felix => "felix",
            Sp::Juno => "juno",
            Sp::Ibrahim => "ibrahim",
            Sp::Lin => "lin",
            Sp::Ada => "ada",
            Sp::Warden => "warden",
            Sp::Agent => "agent",
            Sp::System => "system",
            Sp::Narrator => "narrator",
            Sp::Letter => "letter",
        }
    }

    pub fn info(self, handle: &str, look: usize) -> SpeakerInfo {
        let cast = |name: &str, color: &'static str, id: &str| SpeakerInfo { name: name.into(), color, style: Style::Normal, portrait: Some(id.into()) };
        match self {
            Sp::You => SpeakerInfo {
                name: if handle.is_empty() { "you".into() } else { handle.into() },
                color: "#3ef0ff",
                style: Style::Normal,
                portrait: Some(["player-a", "player-b", "player-c"].get(look).unwrap_or(&"player-a").to_string()),
            },
            Sp::Nonce => SpeakerInfo { name: "Nonce".into(), color: "#a7f070", style: Style::Normal, portrait: Some("cat".into()) },
            Sp::System => SpeakerInfo { name: "TERMINAL".into(), color: "#3ef0ff", style: Style::Terminal, portrait: None },
            Sp::Narrator => SpeakerInfo { name: String::new(), color: "#94b0c2", style: Style::Narration, portrait: None },
            Sp::Letter => SpeakerInfo { name: "Grandpa's letter".into(), color: "#ffcd75", style: Style::Letter, portrait: Some("grandpa".into()) },
            Sp::Agent => cast("DLA Agent", "#ff5566", "agent"),
            Sp::Grandpa => cast("Grandpa", "#ffcd75", "grandpa"),
            Sp::Mara => cast("Mara", "#ffcd75", "mara"),
            Sp::Felix => cast("Felix", "#ef7d57", "felix"),
            Sp::Juno => cast("Juno", "#5dff9a", "juno"),
            Sp::Ibrahim => cast("Ibrahim", "#ff6b81", "ibrahim"),
            Sp::Lin => cast("Lin", "#c792ff", "lin"),
            Sp::Ada => cast("Ada", "#73b7ff", "ada"),
            Sp::Warden => cast("Warden", "#ff5566", "warden"),
        }
    }
}
