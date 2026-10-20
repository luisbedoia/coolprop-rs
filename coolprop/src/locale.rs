/// Language of the display texts in [`schema`](crate::schema) and of
/// [`Phase::label`](crate::Phase::label) / [`Phase::description`](crate::Phase::description).
/// Names, symbols and units never change with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Locale {
    #[default]
    En,
    Es,
}

impl Locale {
    pub const ALL: [Locale; 2] = [Locale::En, Locale::Es];

    /// BCP 47 tag, also the serde JSON name: `"en"`, `"es"`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }

    pub(crate) const fn pick(self, text: Text) -> &'static str {
        match self {
            Self::En => text.en,
            Self::Es => text.es,
        }
    }
}

/// One display text in every locale. A new locale is a new field, so the
/// compiler points at every text still to translate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Text {
    pub(crate) en: &'static str,
    pub(crate) es: &'static str,
}

pub(crate) const fn text(en: &'static str, es: &'static str) -> Text {
    Text { en, es }
}
